//! HyperMesh Runtime Components
//!
//! Provides runtime services and SDKs for HyperMesh applications.

pub mod phoenix;

// Re-export Phoenix SDK types
pub use phoenix::{
    PhoenixTransport, PhoenixConfig, PhoenixConnection,
    PerformanceMetrics, PhoenixBuilder,
};