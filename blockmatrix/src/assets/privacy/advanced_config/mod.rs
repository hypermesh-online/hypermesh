// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy Configuration Module System
//!
//! Provides a modular approach to privacy configuration management,
//! breaking down the complex configuration into focused, manageable modules.

pub mod advanced;
pub mod consent;
pub mod core;
pub mod data_management;
pub mod monitoring;
pub mod resources;
pub mod security;
pub mod sharing;
pub mod templates;
pub mod validation;

// Re-export key types for backward compatibility
pub use advanced::{AdvancedPrivacyOptions, CustomPrivacyAlgorithm};
pub use consent::{ConsentManagementSettings, GranularConsentSettings};
pub use core::{PrivacySettings, UserPrivacyConfig};
pub use data_management::{DataMinimizationSettings, RetentionPreferences};
pub use monitoring::DashboardPreferences;
pub use resources::{ResourceAllocationOptimization, ResourcePrivacySettings};
pub use security::{ArchiveEncryptionSettings, KeyManagementSettings};
pub use sharing::{AnonymizationPreferences, SharingMinimizationSettings};
pub use templates::{PrivacyPreset, PrivacyTemplate};
pub use validation::{PrivacyConstraints, PrivacyValidationRules};
