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

pub mod protocol;
pub mod privacy_filter;
pub mod publisher;
pub mod subscriber;
pub mod aggregator;

// Re-export primary types at module root.
pub use protocol::{MetricsFrame, MetricsPayload, CapacitySnapshot, CongestionSnapshot, RoutingSnapshot, EconomicSnapshot, VerificationSnapshot, ProtocolError};
pub use privacy_filter::DifferentialPrivacyFilter;
pub use publisher::MetricsPublisher;
pub use subscriber::MetricsSubscriber;
pub use aggregator::{RegionalAggregator, RegionalAggregate};
