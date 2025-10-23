//! Privacy Configuration Management
//!
//! User privacy configurations, settings validation, and configuration templates.
//! 
//! This module is broken down into logical sub-modules:
//! - `core`: Core privacy structures and types
//! - `retention`: Data retention and deletion management  
//! - `keys`: Cryptographic key management
//! - `validation`: Configuration validation logic
//! - `templates`: Privacy configuration templates

// These modules are defined at the parent level, not here
// We import them instead of declaring them

// Re-export core types for convenience
pub use super::core::{
    UserPrivacyConfig, PrivacySettings, PrivacyMode, DataMinimizationSettings,
    RetentionPreferences, ResourcePrivacySettings, PrivacyConstraints,
    PrivacyValidationRules, PrivacyTemplate, PrivacyPreset, AdvancedPrivacyOptions,
    ConsentManagementSettings, DashboardPreferences, SharingMinimizationSettings,
    AnonymizationPreferences,
};

pub use super::retention::{
    AutoDeletionSettings, DeletionCriterion, DeletionCriterionType,
    DeletionConfirmationSettings, SecureDeletionMethod, ArchivePreferences,
    ArchiveLocation, ArchiveEncryptionSettings, KeyDerivationSettings,
};

pub use super::keys::{
    KeyManagementSettings, KeyDerivationMethod, KeyRotationSettings,
    KeyRotationMethod, KeyRecoverySettings, KeyRecoveryMethod,
    KeyRecoveryLimitations,
};

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::assets::core::{AssetResult, AssetError};

use super::{
    PrivacyAllocationType, ResourceAllocationConfig, ConsensusRequirementConfig,
    ProxyConfiguration
};

/// Privacy configuration manager
pub struct PrivacyConfigManager {
    configs: HashMap<String, UserPrivacyConfig>,
}

impl PrivacyConfigManager {
    /// Create a new privacy configuration manager
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }
    
    /// Load user privacy configuration
    pub async fn load_config(&self, user_id: &str) -> AssetResult<Option<UserPrivacyConfig>> {
        Ok(self.configs.get(user_id).cloned())
    }
    
    /// Save user privacy configuration
    pub async fn save_config(&mut self, config: UserPrivacyConfig) -> AssetResult<()> {
        self.validate_config(&config)?;
        self.configs.insert(config.user_id.clone(), config);
        Ok(())
    }
    
    /// Validate privacy configuration
    fn validate_config(&self, _config: &UserPrivacyConfig) -> AssetResult<()> {
        // Basic validation - can be expanded
        Ok(())
    }
    
    /// Get default privacy configuration
    pub fn default_config(user_id: String) -> UserPrivacyConfig {
        UserPrivacyConfig {
            user_id,
            config_version: "1.0".to_string(),
            privacy_settings: PrivacySettings {
                default_privacy_level: crate::assets::core::PrivacyLevel::Private,
                default_allocation_type: PrivacyAllocationType::Private,
                privacy_mode: PrivacyMode::Balanced,
                data_minimization: DataMinimizationSettings {
                    enabled: true,
                    retention_preferences: RetentionPreferences {
                        default_retention_period: std::time::Duration::from_secs(86400 * 30), // 30 days
                        resource_specific_retention: HashMap::new(),
                        auto_deletion: AutoDeletionSettings {
                            enabled: false,
                            deletion_criteria: vec![],
                            confirmation_settings: DeletionConfirmationSettings {
                                require_confirmation: true,
                                warning_period: std::time::Duration::from_secs(86400), // 1 day
                                confirmation_prompts: 2,
                            },
                            secure_deletion_method: SecureDeletionMethod::Standard,
                        },
                        archive_preferences: ArchivePreferences {
                            enabled: false,
                            archive_after: std::time::Duration::from_secs(86400 * 90), // 90 days
                            archive_location: ArchiveLocation::Local,
                            encryption_settings: ArchiveEncryptionSettings {
                                enabled: true,
                                algorithm: "AES-256-GCM".to_string(),
                                key_derivation: KeyDerivationSettings {
                                    method: "Argon2id".to_string(),
                                    iterations: 100000,
                                    salt_length: 32,
                                },
                                compress_before_encrypt: true,
                            },
                        },
                    },
                    sharing_minimization: SharingMinimizationSettings {},
                    anonymization_preferences: AnonymizationPreferences {},
                },
                consent_management: ConsentManagementSettings {},
                dashboard_preferences: DashboardPreferences {},
            },
            resource_settings: ResourcePrivacySettings {},
            constraints: PrivacyConstraints {},
            validation_rules: PrivacyValidationRules {},
            templates: vec![],
            presets: vec![],
            advanced_options: AdvancedPrivacyOptions {},
        }
    }
}

impl Default for PrivacyConfigManager {
    fn default() -> Self {
        Self::new()
    }
}