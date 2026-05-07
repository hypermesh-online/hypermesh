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
//! - [`metrics_source`] -- Real system metrics collection (MetricsSource trait).
//! - [`ingestion`] -- MetricsIngestionPipeline for processing MetricsFrame payloads.
//! - [`swarm_analytics`] -- Shard popularity, replication triggers, dispersion (R12).
//! - [`min_spec`] -- Minimum-spec performance profiling (R13).
//! - [`caesar_tracker`] -- Caesar in-transit/holding amount tracking.
//! - [`collective_intel`] -- Collective network intelligence aggregation.

pub mod api;
pub mod caesar_tracker;
pub mod capacity;
pub mod collective_intel;
pub mod compliance;
pub mod ingestion;
pub mod marketplace;
pub mod metrics;
pub mod metrics_source;
pub mod min_spec;
pub mod node_metrics;
pub mod organic_detection;
pub mod receipt;
pub mod routing_intel;
pub mod streaming;
pub mod swarm_analytics;
pub mod throttle;
pub mod trending;
pub mod trust_signals;
pub mod udp_ingest;

// Re-export primary types at crate root for convenience.
pub use capacity::{CapacityMetrics, CapacityReport, CapacityScore};
pub use caesar_tracker::CaesarTracker;
pub use collective_intel::CollectiveIntelligence;
pub use compliance::{AttestationLevel, ComplianceChecker, ComplianceResult, KycAttestation};
pub use ingestion::MetricsIngestionPipeline;
pub use marketplace::{
    ContentPushManager, LeaseContract, LeaseManager, PricingEngine, ResourcePool,
};
pub use metrics::{ActivityScore, MetricsCollector, MetricsSnapshot};
pub use metrics_source::{MetricsSource, MockMetricsSource, SystemMetricsSource};
pub use min_spec::{MinSpecProfiler, ResourceUsage};
pub use node_metrics::{
    assess_capacity, CapacityLevel, HardwareSummary, PeerMetrics, SelfMetrics, TransportSummary,
};
pub use organic_detection::{
    ClassifierConfig, TrafficClassification, TrafficClassifier, TrafficPattern,
};
pub use receipt::{ContentReceipt, ReceiptBundle, VerificationResult, WorkUnits};
pub use routing_intel::{
    EbpfPolicyFeedback, EbpfPrivacyAction, EbpfRoutingRule, PathAdvisor, PathPolicyRecommendation,
    RoutingAdvisor, RoutingIntelFeed, RoutingIntelligence, RoutingUpdate, TensorWeightModifier,
};
pub use streaming::{
    MetricsFrame, MetricsPayload, MetricsPublisher, MetricsSubscriber, RegionalAggregator,
};
pub use swarm_analytics::{
    CascadeTracker, DispersionAdvisor, ReplicationConfig, ReplicationRecommendation,
    ReplicationSignal, ReplicationTrigger, SwarmAnalytics,
};
pub use throttle::{EngaugeThrottle, ThrottleSignal};
pub use trending::{AggregatedCapacity, CapacityTrend, EpochRecord, EpochTracker, TrendDirection};
pub use trust_signals::{PeerTrustSignals, TrustBand};
