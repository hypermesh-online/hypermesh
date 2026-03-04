// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Performance Monitoring Dashboard
//!
//! This module provides a comprehensive performance monitoring dashboard for HyperMesh
//! infrastructure, including real-time container performance metrics, P2P network
//! monitoring, Byzantine fault tolerance metrics, and Proof of State performance analysis.
//!
//! # Features
//!
//! - Real-time performance metrics collection and visualization
//! - Container startup and scaling performance tracking (<100ms targets)
//! - P2P mesh connectivity performance (<5ms connection establishment)
//! - Byzantine fault detection and reputation tracking
//! - State proof latency monitoring (<50ms coordination overhead)
//! - Network throughput and utilization analysis
//! - Automated performance alerting and remediation triggers

// Re-export the performance dashboard module
pub mod performance;

// Re-export key types for backwards compatibility
pub use performance::{
    PerformanceDashboard,
    DashboardConfig,
    DashboardData,
    ExportFormat,
    PerformanceThresholds,
    AggregatedMetrics,
};
