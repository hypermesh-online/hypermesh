//! Core Privacy Configuration Types
//!
//! Fundamental privacy configuration structures and enums that form
//! the foundation of the privacy management system.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetResult, AssetError, PrivacyLevel};
use super::super::{PrivacyAllocationType, ResourceAllocationConfig, ConsensusRequirementConfig, ProxyConfiguration};

// Re-export sub-modules
pub use super::data_management::{DataMinimizationSettings, RetentionPreferences};
pub use super::consent::ConsentManagementSettings;
pub use super::monitoring::DashboardPreferences;
pub use super::resources::ResourcePrivacySettings;
pub use super::validation::{PrivacyConstraints, PrivacyValidationRules};
pub use super::templates::{PrivacyTemplate, PrivacyPreset};
pub use super::advanced::AdvancedPrivacyOptions;

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

impl UserPrivacyConfig {
    /// Create new privacy configuration for user
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            config_version: "1.0.0".to_string(),
            privacy_settings: PrivacySettings::default(),
            resource_settings: ResourcePrivacySettings::default(),
            constraints: PrivacyConstraints::default(),
            validation_rules: PrivacyValidationRules::default(),
            templates: Vec::new(),
            presets: Vec::new(),
            advanced_options: AdvancedPrivacyOptions::default(),
        }
    }

    /// Validate configuration consistency
    pub fn validate(&self) -> AssetResult<()> {
        // Validate user ID
        if self.user_id.trim().is_empty() {
            return Err(AssetError::ValidationError(
                "User ID cannot be empty".to_string()
            ));
        }

        // Validate configuration version
        if self.config_version.trim().is_empty() {
            return Err(AssetError::ValidationError(
                "Configuration version cannot be empty".to_string()
            ));
        }

        // Validate privacy settings
        self.privacy_settings.validate()?;
        
        // Validate constraints
        self.constraints.validate()?;
        
        // Validate validation rules
        self.validation_rules.validate()?;

        Ok(())
    }

    /// Apply privacy template
    pub fn apply_template(&mut self, template_name: &str) -> AssetResult<()> {
        let template = self.templates.iter()
            .find(|t| t.name == template_name)
            .ok_or_else(|| AssetError::NotFound(
                format!("Privacy template '{}' not found", template_name)
            ))?;

        template.apply_to_config(self)?;
        Ok(())
    }

    /// Apply privacy preset
    pub fn apply_preset(&mut self, preset_name: &str) -> AssetResult<()> {
        let preset = self.presets.iter()
            .find(|p| p.name == preset_name)
            .ok_or_else(|| AssetError::NotFound(
                format!("Privacy preset '{}' not found", preset_name)
            ))?;

        preset.apply_to_config(self)?;
        Ok(())
    }
}

impl PrivacySettings {
    /// Validate privacy settings
    pub fn validate(&self) -> AssetResult<()> {
        // Validate data minimization settings
        self.data_minimization.validate()?;
        
        // Validate consent management settings
        self.consent_management.validate()?;
        
        Ok(())
    }
}

impl Default for UserPrivacyConfig {
    fn default() -> Self {
        Self::new("default_user".to_string())
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            default_privacy_level: PrivacyLevel::Private,
            default_allocation_type: PrivacyAllocationType::Private,
            privacy_mode: PrivacyMode::Balanced,
            data_minimization: DataMinimizationSettings::default(),
            consent_management: ConsentManagementSettings::default(),
            dashboard_preferences: DashboardPreferences::default(),
        }
    }
}

impl Default for PrivacyMode {
    fn default() -> Self {
        PrivacyMode::Balanced
    }
}