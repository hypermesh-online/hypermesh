//! Core privacy configuration structures
//!
//! Base privacy settings and fundamental types.

use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::assets::core::PrivacyLevel;

use super::{
    PrivacyAllocationType, ResourceAllocationConfig, ConsensusRequirementConfig,
    ProxyConfiguration
};

/// Complete user privacy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPrivacyConfig {
    /// User identifier
    pub user_id: String,
    
    /// Configuration version
    pub config_version: String,
    
    /// Main privacy settings
    pub privacy_settings: PrivacySettings,
    
    /// Resource-specific privacy settings
    pub resource_settings: ResourcePrivacySettings,
    
    /// Privacy constraints and limits
    pub constraints: PrivacyConstraints,
    
    /// Validation rules
    pub validation_rules: PrivacyValidationRules,
    
    /// Configuration templates
    pub templates: Vec<PrivacyTemplate>,
    
    /// Quick settings presets
    pub presets: Vec<PrivacyPreset>,
    
    /// Advanced configuration options
    pub advanced_options: AdvancedPrivacyOptions,
}

/// Main privacy settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Default privacy level
    pub default_privacy_level: PrivacyLevel,
    
    /// Default allocation type
    pub default_allocation_type: PrivacyAllocationType,
    
    /// Privacy mode preferences
    pub privacy_mode: PrivacyMode,
    
    /// Data minimization settings
    pub data_minimization: DataMinimizationSettings,
    
    /// Consent management
    pub consent_management: ConsentManagementSettings,
    
    /// Privacy dashboard preferences
    pub dashboard_preferences: DashboardPreferences,
}

/// Privacy mode options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrivacyMode {
    /// Maximum privacy protection
    MaximumPrivacy,
    /// Balance privacy and functionality
    Balanced,
    /// Maximum functionality with minimal privacy
    MaximumFunctionality,
    /// Custom configuration
    Custom,
}

/// Data minimization settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataMinimizationSettings {
    /// Enable automatic data minimization
    pub enabled: bool,
    
    /// Data retention preferences
    pub retention_preferences: RetentionPreferences,
    
    /// Data sharing minimization
    pub sharing_minimization: SharingMinimizationSettings,
    
    /// Anonymization preferences
    pub anonymization_preferences: AnonymizationPreferences,
}

/// Data retention preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionPreferences {
    /// Default retention period
    pub default_retention_period: Duration,
    
    /// Resource-specific retention periods
    pub resource_specific_retention: std::collections::HashMap<String, Duration>,
    
    /// Auto-deletion settings
    pub auto_deletion: AutoDeletionSettings,
    
    /// Archive preferences
    pub archive_preferences: ArchivePreferences,
}

// Stub structures to be moved to appropriate modules
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourcePrivacySettings {
    // TODO: Move to resource_settings.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyConstraints {
    // TODO: Move to constraints.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyValidationRules {
    // TODO: Move to validation.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyTemplate {
    // TODO: Move to templates.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyPreset {
    // TODO: Move to presets.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdvancedPrivacyOptions {
    // TODO: Move to advanced.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentManagementSettings {
    // TODO: Move to consent.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardPreferences {
    // TODO: Move to dashboard.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingMinimizationSettings {
    // TODO: Move to sharing.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymizationPreferences {
    // TODO: Move to anonymization.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoDeletionSettings {
    // TODO: Move to deletion.rs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchivePreferences {
    // TODO: Move to archive.rs
}