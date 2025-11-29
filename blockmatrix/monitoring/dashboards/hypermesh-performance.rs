//! HyperMesh Performance Monitoring Dashboard
//!
//! This module provides a comprehensive performance monitoring dashboard for HyperMesh
//! infrastructure, including real-time container performance metrics, P2P network
//! monitoring, Byzantine fault tolerance metrics, and consensus performance analysis.
//!
//! # Features
//!
//! - Real-time performance metrics collection and visualization
//! - Container startup and scaling performance tracking (<100ms targets)
//! - P2P mesh connectivity performance (<5ms connection establishment)
//! - Byzantine fault detection and reputation tracking
//! - Consensus latency monitoring (<50ms coordination overhead)
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
