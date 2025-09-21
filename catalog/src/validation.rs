//! Asset Validation Framework (Compatibility Layer)
//!
//! This file maintains backward compatibility after refactoring into modules.
//! All validation functionality has been reorganized into the validation/ subdirectory
//! following the 500/50/3 rule.

// Re-export all public items from the validation module
pub use validation::*;