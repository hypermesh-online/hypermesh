//! Privacy Configuration Module System
//!
//! Provides a modular approach to privacy configuration management,
//! breaking down the complex configuration into focused, manageable modules.

pub mod core;
pub mod data_management;
pub mod sharing;
pub mod security;
pub mod consent;
pub mod monitoring;
pub mod resources;
pub mod validation;
pub mod templates;
pub mod advanced;

// Re-export key types for backward compatibility
pub use core::{UserPrivacyConfig, PrivacySettings};
pub use data_management::{DataMinimizationSettings, RetentionPreferences};
pub use sharing::{SharingMinimizationSettings, AnonymizationPreferences};
pub use security::{ArchiveEncryptionSettings, KeyManagementSettings};
pub use consent::{ConsentManagementSettings, GranularConsentSettings};
pub use monitoring::{DashboardPreferences};
pub use resources::{ResourcePrivacySettings, ResourceAllocationOptimization};
pub use validation::{PrivacyValidationRules, PrivacyConstraints};
pub use templates::{PrivacyTemplate, PrivacyPreset};
pub use advanced::{AdvancedPrivacyOptions, CustomPrivacyAlgorithm};