// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Engauge -- work tracking, content receipts, organic/speculative detection,
//! and capacity metrics for the HyperMesh ecosystem.
//!
//! # Modules
//!
//! - [`receipt`] -- BLAKE3-hashed content receipts proving work was done.
//! - [`metrics`] -- Per-node activity metrics and scoring for Governor data feed.
//! - [`compliance`] -- Self-sovereign KYC attestation (hash-only, no PII).
//! - [`organic_detection`] -- Organic vs speculative traffic pattern classification.
//! - [`throttle`] -- Governor feedback signals (band/demurrage modifiers).
//! - [`capacity`] -- Per-node capacity metrics (bytes served, compute, uptime).

pub mod capacity;
pub mod compliance;
pub mod marketplace;
pub mod metrics;
pub mod organic_detection;
pub mod receipt;
pub mod routing_intel;
pub mod streaming;
pub mod throttle;
pub mod trending;

// Re-export primary types at crate root for convenience.
pub use capacity::{CapacityMetrics, CapacityReport, CapacityScore};
pub use compliance::{AttestationLevel, ComplianceChecker, ComplianceResult, KycAttestation};
pub use marketplace::{
    ContentPushManager, LeaseContract, LeaseManager, PricingEngine, ResourcePool,
};
pub use metrics::{ActivityScore, MetricsCollector, MetricsSnapshot};
pub use organic_detection::{
    ClassifierConfig, TrafficClassification, TrafficClassifier, TrafficPattern,
};
pub use receipt::{ContentReceipt, ReceiptBundle, VerificationResult, WorkUnits};
pub use routing_intel::{
    PathAdvisor, PathPolicyRecommendation, RoutingAdvisor, RoutingIntelligence,
    TensorWeightModifier,
};
pub use streaming::{
    MetricsFrame, MetricsPayload, MetricsPublisher, MetricsSubscriber, RegionalAggregator,
};
pub use throttle::{EngaugeThrottle, ThrottleSignal};
pub use trending::{AggregatedCapacity, CapacityTrend, EpochRecord, EpochTracker, TrendDirection};
