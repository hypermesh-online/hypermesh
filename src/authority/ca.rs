//! Embedded Certificate Authority (Compatibility Layer)
//!
//! This file maintains backward compatibility after refactoring into modules.
//! All CA functionality has been reorganized into the ca/ subdirectory
//! following the 500/50/3 rule.

// Re-export all public items from the ca module
pub use ca::*;