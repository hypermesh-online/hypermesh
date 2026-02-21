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

pub mod receipt;
pub mod metrics;
pub mod compliance;
pub mod organic_detection;
pub mod throttle;
pub mod capacity;
pub mod trending;
pub mod streaming;
pub mod routing_intel;
pub mod marketplace;

// Re-export primary types at crate root for convenience.
pub use receipt::{ContentReceipt, ReceiptBundle, WorkUnits, VerificationResult};
pub use metrics::{MetricsCollector, ActivityScore, MetricsSnapshot};
pub use compliance::{KycAttestation, AttestationLevel, ComplianceChecker, ComplianceResult};
pub use organic_detection::{TrafficPattern, TrafficClassification, TrafficClassifier, ClassifierConfig};
pub use throttle::{ThrottleSignal, EngaugeThrottle};
pub use capacity::{CapacityMetrics, CapacityScore, CapacityReport};
pub use trending::{EpochTracker, EpochRecord, TrendDirection, CapacityTrend, AggregatedCapacity};
pub use streaming::{MetricsFrame, MetricsPayload, MetricsPublisher, MetricsSubscriber, RegionalAggregator};
pub use routing_intel::{RoutingIntelligence, RoutingAdvisor, PathAdvisor, TensorWeightModifier, PathPolicyRecommendation};
pub use marketplace::{ResourcePool, LeaseContract, LeaseManager, PricingEngine, ContentPushManager};
