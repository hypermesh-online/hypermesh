//! Data Management Configuration
//!
//! Configuration for data minimization, retention, deletion, and archival policies.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetResult, AssetError};
use super::sharing::{SharingMinimizationSettings, AnonymizationPreferences};

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
    
    /// Per-data-type retention settings
    pub per_type_retention: HashMap<String, Duration>,
    
    /// Auto-deletion settings
    pub auto_deletion: AutoDeletionSettings,
    
    /// Archive preferences
    pub archive_preferences: ArchivePreferences,
}

/// Auto-deletion settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoDeletionSettings {
    /// Enable automatic deletion
    pub enabled: bool,
    
    /// Deletion criteria
    pub criteria: Vec<DeletionCriterion>,
    
    /// Deletion confirmation requirements
    pub confirmation_requirements: DeletionConfirmationSettings,
    
    /// Secure deletion method
    pub deletion_method: SecureDeletionMethod,
}

/// Deletion criteria
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeletionCriterion {
    /// Criterion name
    pub name: String,
    
    /// Criterion type
    pub criterion_type: DeletionCriterionType,
    
    /// Threshold values
    pub thresholds: HashMap<String, String>,
    
    /// Priority level
    pub priority: u32,
}

/// Types of deletion criteria
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeletionCriterionType {
    TimeBasedExpiry,
    UsageBasedExpiry,
    StorageThreshold,
    ComplianceRequirement,
    UserRequest,
}

/// Deletion confirmation settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeletionConfirmationSettings {
    /// Require user confirmation
    pub require_user_confirmation: bool,
    
    /// Confirmation timeout
    pub confirmation_timeout: Duration,
    
    /// Multi-factor confirmation
    pub require_mfa_confirmation: bool,
    
    /// Grace period before deletion
    pub grace_period: Duration,
}

/// Secure deletion methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SecureDeletionMethod {
    StandardDeletion,
    SecureOverwrite,
    CryptographicErasure,
    PhysicalDestruction,
}

/// Archive preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchivePreferences {
    /// Enable archiving before deletion
    pub enable_archiving: bool,
    
    /// Archive storage location
    pub archive_location: ArchiveLocation,
    
    /// Archive encryption settings
    pub encryption_settings: ArchiveEncryptionSettings,
    
    /// Archive access controls
    pub access_controls: ArchiveAccessControls,
}

/// Archive storage location options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ArchiveLocation {
    LocalStorage,
    CloudStorage { provider: String, region: String },
    DistributedStorage,
    UserControlledStorage { location: String },
}

/// Archive encryption settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveEncryptionSettings {
    /// Encryption enabled
    pub enabled: bool,
    
    /// Encryption algorithm
    pub algorithm: String,
    
    /// Key management
    pub key_management: KeyManagementSettings,
    
    /// Additional security measures
    pub additional_security: Vec<String>,
}

/// Key management settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyManagementSettings {
    /// Key derivation method
    pub key_derivation: KeyDerivationMethod,
    
    /// Key rotation settings
    pub key_rotation: KeyRotationSettings,
    
    /// Key recovery options
    pub key_recovery: KeyRecoverySettings,
}

/// Key derivation methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KeyDerivationMethod {
    UserPassword,
    HardwareToken,
    BiometricData,
    ConsensusProof,
    MultiParty,
}

/// Key rotation settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRotationSettings {
    /// Enable automatic key rotation
    pub enabled: bool,
    
    /// Rotation frequency
    pub frequency: Duration,
    
    /// Trigger conditions
    pub trigger_conditions: Vec<String>,
    
    /// Rotation method
    pub rotation_method: KeyRotationMethod,
}

/// Key rotation methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KeyRotationMethod {
    Automatic,
    Manual,
    EventTriggered,
    TimeTriggered,
}

/// Key recovery settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRecoverySettings {
    /// Enable key recovery
    pub enabled: bool,
    
    /// Recovery methods
    pub recovery_methods: Vec<KeyRecoveryMethod>,
    
    /// Recovery verification
    pub verification_requirements: Vec<String>,
    
    /// Recovery limitations
    pub limitations: KeyRecoveryLimitations,
}

/// Key recovery methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KeyRecoveryMethod {
    BackupPhrase,
    SecretSharing,
    TrustedContacts,
    BiometricRecovery,
    HardwareBackup,
}

/// Key recovery limitations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRecoveryLimitations {
    /// Maximum recovery attempts
    pub max_attempts: u32,
    
    /// Recovery timeout
    pub recovery_timeout: Duration,
    
    /// Cooling off period
    pub cooloff_period: Duration,
    
    /// Verification escalation
    pub escalation_requirements: Vec<String>,
}

/// Archive access controls
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveAccessControls {
    /// Access permissions
    pub permissions: ArchivePermissions,
    
    /// Access logging
    pub access_logging: ArchiveAccessLogging,
    
    /// Access restrictions
    pub restrictions: Vec<ArchiveAccessRestriction>,
}

/// Archive permissions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchivePermissions {
    /// User can access own archives
    pub user_access: bool,
    
    /// Admin access permissions
    pub admin_access: AdminAccessPermissions,
    
    /// Legal access provisions
    pub legal_access: LegalAccessProvisions,
    
    /// Emergency access procedures
    pub emergency_access: EmergencyAccessProcedures,
}

/// Administrator access permissions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminAccessPermissions {
    /// Allow admin access
    pub allowed: bool,
    
    /// Justification requirements
    pub justification_required: bool,
    
    /// Audit trail requirements
    pub audit_trail_required: bool,
    
    /// User notification requirements
    pub notify_user: bool,
}

/// Legal access provisions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegalAccessProvisions {
    /// Compliance with legal requests
    pub legal_compliance: bool,
    
    /// Jurisdiction restrictions
    pub jurisdiction_restrictions: Vec<String>,
    
    /// Legal process requirements
    pub process_requirements: Vec<String>,
    
    /// User notification policies
    pub notification_policies: LegalNotificationPolicies,
}

/// Legal notification policies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegalNotificationPolicies {
    /// Notify user of legal requests
    pub notify_user: bool,
    
    /// Notification delay allowances
    pub delay_allowances: HashMap<String, Duration>,
    
    /// Exception circumstances
    pub exceptions: Vec<String>,
}

/// Emergency access procedures
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergencyAccessProcedures {
    /// Emergency conditions
    pub conditions: Vec<EmergencyCondition>,
    
    /// Access limitations during emergency
    pub limitations: EmergencyAccessLimitations,
}

/// Emergency condition definitions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergencyCondition {
    /// Condition name
    pub name: String,
    
    /// Condition type
    pub condition_type: EmergencyConditionType,
    
    /// Trigger criteria
    pub trigger_criteria: HashMap<String, String>,
    
    /// Duration of emergency status
    pub duration: Duration,
}

/// Types of emergency conditions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EmergencyConditionType {
    SystemFailure,
    SecurityBreach,
    LegalCompliance,
    UserSafety,
    DataCorruption,
}

/// Emergency access limitations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergencyAccessLimitations {
    /// Time-based limitations
    pub time_limits: HashMap<String, Duration>,
    
    /// Scope limitations
    pub scope_limitations: Vec<String>,
    
    /// Approval requirements
    pub approval_requirements: Vec<String>,
}

/// Archive access logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveAccessLogging {
    /// Enable access logging
    pub enabled: bool,
    
    /// Log detail level
    pub detail_level: LogDetailLevel,
    
    /// Log retention period
    pub retention_period: Duration,
    
    /// Log security settings
    pub security_settings: LogSecuritySettings,
}

/// Log detail levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogDetailLevel {
    Minimal,
    Standard,
    Detailed,
    Comprehensive,
}

/// Log security settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogSecuritySettings {
    /// Encrypt logs
    pub encryption_enabled: bool,
    
    /// Immutable logging
    pub immutable_logging: bool,
    
    /// Tamper detection
    pub tamper_detection: bool,
    
    /// Access control for logs
    pub log_access_control: bool,
}

/// Archive access restriction
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveAccessRestriction {
    /// Restriction name
    pub name: String,
    
    /// Restriction type
    pub restriction_type: ArchiveRestrictionType,
    
    /// Restriction parameters
    pub parameters: HashMap<String, String>,
    
    /// Override conditions
    pub override_conditions: Vec<String>,
}

/// Types of archive access restrictions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ArchiveRestrictionType {
    TimeWindow,
    LocationBased,
    DeviceBased,
    UserRoleBased,
    ComplianceBased,
}

// Validation implementations
impl DataMinimizationSettings {
    pub fn validate(&self) -> AssetResult<()> {
        self.retention_preferences.validate()?;
        self.sharing_minimization.validate()?;
        self.anonymization_preferences.validate()?;
        Ok(())
    }
}

impl RetentionPreferences {
    pub fn validate(&self) -> AssetResult<()> {
        if self.default_retention_period.as_secs() == 0 {
            return Err(AssetError::ValidationError(
                "Default retention period cannot be zero".to_string()
            ));
        }
        
        self.auto_deletion.validate()?;
        self.archive_preferences.validate()?;
        Ok(())
    }
}

impl AutoDeletionSettings {
    pub fn validate(&self) -> AssetResult<()> {
        if self.enabled && self.criteria.is_empty() {
            return Err(AssetError::ValidationError(
                "Auto-deletion enabled but no criteria specified".to_string()
            ));
        }
        
        for criterion in &self.criteria {
            criterion.validate()?;
        }
        
        Ok(())
    }
}

impl DeletionCriterion {
    pub fn validate(&self) -> AssetResult<()> {
        if self.name.trim().is_empty() {
            return Err(AssetError::ValidationError(
                "Deletion criterion name cannot be empty".to_string()
            ));
        }
        
        if self.thresholds.is_empty() {
            return Err(AssetError::ValidationError(
                "Deletion criterion must have at least one threshold".to_string()
            ));
        }
        
        Ok(())
    }
}

impl ArchivePreferences {
    pub fn validate(&self) -> AssetResult<()> {
        if self.enable_archiving {
            self.encryption_settings.validate()?;
            self.access_controls.validate()?;
        }
        Ok(())
    }
}

impl ArchiveEncryptionSettings {
    pub fn validate(&self) -> AssetResult<()> {
        if self.enabled {
            if self.algorithm.trim().is_empty() {
                return Err(AssetError::ValidationError(
                    "Encryption algorithm must be specified when encryption is enabled".to_string()
                ));
            }
            self.key_management.validate()?;
        }
        Ok(())
    }
}

impl KeyManagementSettings {
    pub fn validate(&self) -> AssetResult<()> {
        self.key_rotation.validate()?;
        self.key_recovery.validate()?;
        Ok(())
    }
}

impl KeyRotationSettings {
    pub fn validate(&self) -> AssetResult<()> {
        if self.enabled && self.frequency.as_secs() == 0 {
            return Err(AssetError::ValidationError(
                "Key rotation frequency cannot be zero when enabled".to_string()
            ));
        }
        Ok(())
    }
}

impl KeyRecoverySettings {
    pub fn validate(&self) -> AssetResult<()> {
        if self.enabled && self.recovery_methods.is_empty() {
            return Err(AssetError::ValidationError(
                "Key recovery enabled but no recovery methods specified".to_string()
            ));
        }
        Ok(())
    }
}

impl ArchiveAccessControls {
    pub fn validate(&self) -> AssetResult<()> {
        // Validation logic for access controls
        Ok(())
    }
}

// Default implementations
impl Default for DataMinimizationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_preferences: RetentionPreferences::default(),
            sharing_minimization: SharingMinimizationSettings::default(),
            anonymization_preferences: AnonymizationPreferences::default(),
        }
    }
}

impl Default for RetentionPreferences {
    fn default() -> Self {
        Self {
            default_retention_period: Duration::from_secs(30 * 24 * 3600), // 30 days
            per_type_retention: HashMap::new(),
            auto_deletion: AutoDeletionSettings::default(),
            archive_preferences: ArchivePreferences::default(),
        }
    }
}

impl Default for AutoDeletionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            criteria: Vec::new(),
            confirmation_requirements: DeletionConfirmationSettings::default(),
            deletion_method: SecureDeletionMethod::StandardDeletion,
        }
    }
}

impl Default for DeletionConfirmationSettings {
    fn default() -> Self {
        Self {
            require_user_confirmation: true,
            confirmation_timeout: Duration::from_secs(24 * 3600), // 24 hours
            require_mfa_confirmation: false,
            grace_period: Duration::from_secs(7 * 24 * 3600), // 7 days
        }
    }
}

impl Default for ArchivePreferences {
    fn default() -> Self {
        Self {
            enable_archiving: false,
            archive_location: ArchiveLocation::LocalStorage,
            encryption_settings: ArchiveEncryptionSettings::default(),
            access_controls: ArchiveAccessControls::default(),
        }
    }
}

impl Default for ArchiveEncryptionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: "AES-256-GCM".to_string(),
            key_management: KeyManagementSettings::default(),
            additional_security: Vec::new(),
        }
    }
}

impl Default for KeyManagementSettings {
    fn default() -> Self {
        Self {
            key_derivation: KeyDerivationMethod::UserPassword,
            key_rotation: KeyRotationSettings::default(),
            key_recovery: KeyRecoverySettings::default(),
        }
    }
}

impl Default for KeyRotationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            frequency: Duration::from_secs(90 * 24 * 3600), // 90 days
            trigger_conditions: Vec::new(),
            rotation_method: KeyRotationMethod::Automatic,
        }
    }
}

impl Default for KeyRecoverySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            recovery_methods: Vec::new(),
            verification_requirements: Vec::new(),
            limitations: KeyRecoveryLimitations::default(),
        }
    }
}

impl Default for KeyRecoveryLimitations {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            recovery_timeout: Duration::from_secs(3600), // 1 hour
            cooloff_period: Duration::from_secs(24 * 3600), // 24 hours
            escalation_requirements: Vec::new(),
        }
    }
}

impl Default for ArchiveAccessControls {
    fn default() -> Self {
        Self {
            permissions: ArchivePermissions::default(),
            access_logging: ArchiveAccessLogging::default(),
            restrictions: Vec::new(),
        }
    }
}

impl Default for ArchivePermissions {
    fn default() -> Self {
        Self {
            user_access: true,
            admin_access: AdminAccessPermissions::default(),
            legal_access: LegalAccessProvisions::default(),
            emergency_access: EmergencyAccessProcedures::default(),
        }
    }
}

impl Default for AdminAccessPermissions {
    fn default() -> Self {
        Self {
            allowed: false,
            justification_required: true,
            audit_trail_required: true,
            notify_user: true,
        }
    }
}

impl Default for LegalAccessProvisions {
    fn default() -> Self {
        Self {
            legal_compliance: false,
            jurisdiction_restrictions: Vec::new(),
            process_requirements: Vec::new(),
            notification_policies: LegalNotificationPolicies::default(),
        }
    }
}

impl Default for LegalNotificationPolicies {
    fn default() -> Self {
        Self {
            notify_user: true,
            delay_allowances: HashMap::new(),
            exceptions: Vec::new(),
        }
    }
}

impl Default for EmergencyAccessProcedures {
    fn default() -> Self {
        Self {
            conditions: Vec::new(),
            limitations: EmergencyAccessLimitations::default(),
        }
    }
}

impl Default for EmergencyAccessLimitations {
    fn default() -> Self {
        Self {
            time_limits: HashMap::new(),
            scope_limitations: Vec::new(),
            approval_requirements: Vec::new(),
        }
    }
}

impl Default for ArchiveAccessLogging {
    fn default() -> Self {
        Self {
            enabled: true,
            detail_level: LogDetailLevel::Standard,
            retention_period: Duration::from_secs(365 * 24 * 3600), // 1 year
            security_settings: LogSecuritySettings::default(),
        }
    }
}

impl Default for LogSecuritySettings {
    fn default() -> Self {
        Self {
            encryption_enabled: true,
            immutable_logging: true,
            tamper_detection: true,
            log_access_control: true,
        }
    }
}