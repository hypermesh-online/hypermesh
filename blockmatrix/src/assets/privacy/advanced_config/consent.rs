//! Consent Management Configuration
//!
//! Configuration for user consent collection, management, and verification.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetResult, AssetError};

/// Consent management settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentManagementSettings {
    /// Consent model
    pub consent_model: ConsentModel,
    
    /// Granular consent settings
    pub granular_consent: GranularConsentSettings,
    
    /// Consent withdrawal settings
    pub withdrawal_settings: ConsentWithdrawalSettings,
    
    /// Consent verification settings
    pub verification_settings: ConsentVerificationSettings,
    
    /// Consent audit trail settings
    pub audit_trail: ConsentAuditTrailSettings,
}

/// Consent models
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsentModel {
    OptIn,
    OptOut,
    Explicit,
    Implied,
    Granular,
}

/// Granular consent settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GranularConsentSettings {
    /// Enable granular consent
    pub enabled: bool,
    
    /// Consent categories
    pub categories: Vec<ConsentCategory>,
    
    /// Category dependencies
    pub dependencies: HashMap<String, Vec<String>>,
    
    /// Default consent states
    pub default_states: HashMap<String, ConsentState>,
}

/// Consent category
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentCategory {
    /// Category identifier
    pub id: String,
    
    /// Category name
    pub name: String,
    
    /// Category description
    pub description: String,
    
    /// Required vs optional
    pub required: bool,
    
    /// Default state
    pub default_state: ConsentState,
    
    /// Sub-categories
    pub sub_categories: Vec<ConsentCategory>,
}

/// Consent states
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsentState {
    Granted,
    Denied,
    Pending,
    Withdrawn,
    Expired,
}

/// Consent withdrawal settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentWithdrawalSettings {
    /// Withdrawal methods
    pub withdrawal_methods: Vec<WithdrawalMethod>,
    
    /// Confirmation settings
    pub confirmation_settings: WithdrawalConfirmationSettings,
    
    /// Grace period settings
    pub grace_period_settings: WithdrawalGracePeriodSettings,
    
    /// Processing time limits
    pub processing_time_limits: HashMap<String, Duration>,
}

/// Withdrawal methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WithdrawalMethod {
    SelfService,
    Email,
    Phone,
    WrittenRequest,
    InPerson,
}

/// Withdrawal confirmation settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WithdrawalConfirmationSettings {
    /// Require confirmation
    pub require_confirmation: bool,
    
    /// Confirmation methods
    pub confirmation_methods: Vec<String>,
    
    /// Confirmation timeout
    pub confirmation_timeout: Duration,
    
    /// Multiple confirmations required
    pub multiple_confirmations: bool,
}

/// Withdrawal grace period settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WithdrawalGracePeriodSettings {
    /// Grace period duration
    pub duration: Duration,
    
    /// Grace period notifications
    pub notifications: Vec<GracePeriodNotification>,
    
    /// Allow cancellation during grace period
    pub allow_cancellation: bool,
    
    /// Automatic processing after grace period
    pub automatic_processing: bool,
}

/// Grace period notification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GracePeriodNotification {
    /// Notification timing
    pub timing: NotificationTiming,
    
    /// Notification content
    pub content: String,
    
    /// Notification methods
    pub methods: Vec<String>,
    
    /// Require acknowledgment
    pub require_acknowledgment: bool,
}

/// Notification timing options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationTiming {
    Immediate,
    DaysBeforeExpiry(u32),
    HoursBeforeExpiry(u32),
    AtGracePeriodStart,
    AtGracePeriodEnd,
}

/// Consent verification settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentVerificationSettings {
    /// Verification requirements
    pub requirements: ConsentVerificationRequirements,
    
    /// Re-verification settings
    pub reverification: ConsentReverificationSettings,
    
    /// Verification methods
    pub verification_methods: Vec<String>,
    
    /// Verification frequency
    pub verification_frequency: Duration,
}

/// Consent verification requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentVerificationRequirements {
    /// Identity verification required
    pub identity_verification: bool,
    
    /// Age verification required
    pub age_verification: bool,
    
    /// Capacity verification required
    pub capacity_verification: bool,
    
    /// Documentation requirements
    pub documentation_requirements: Vec<String>,
}

/// Consent re-verification settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentReverificationSettings {
    /// Enable re-verification
    pub enabled: bool,
    
    /// Re-verification triggers
    pub triggers: Vec<ReverificationTrigger>,
    
    /// Re-verification frequency
    pub frequency: Duration,
    
    /// Grace period for re-verification
    pub grace_period: Duration,
}

/// Re-verification triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReverificationTrigger {
    TimeExpiry,
    PolicyChange,
    DataSensitivityIncrease,
    RegulatoryRequirement,
    UserRequest,
}

/// Consent audit trail settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentAuditTrailSettings {
    /// Enable audit trail
    pub enabled: bool,
    
    /// Audit events to track
    pub tracked_events: Vec<ConsentAuditEvent>,
    
    /// Audit retention period
    pub retention_period: Duration,
    
    /// Audit security settings
    pub security_settings: HashMap<String, String>,
    
    /// Real-time audit alerts
    pub realtime_alerts: bool,
}

/// Consent audit events
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsentAuditEvent {
    ConsentGranted,
    ConsentDenied,
    ConsentWithdrawn,
    ConsentModified,
    ConsentExpired,
    ConsentReverified,
    ConsentRequested,
}

impl ConsentManagementSettings {
    pub fn validate(&self) -> AssetResult<()> {
        self.granular_consent.validate()?;
        self.withdrawal_settings.validate()?;
        self.verification_settings.validate()?;
        Ok(())
    }
}

impl GranularConsentSettings {
    pub fn validate(&self) -> AssetResult<()> {
        if self.enabled && self.categories.is_empty() {
            return Err(AssetError::ValidationError { message: "Granular consent enabled but no categories specified".to_string() });
        }
        
        for category in &self.categories {
            category.validate()?;
        }
        
        Ok(())
    }
}

impl ConsentCategory {
    pub fn validate(&self) -> AssetResult<()> {
        if self.id.trim().is_empty() {
            return Err(AssetError::ValidationError { message: "Consent category ID cannot be empty".to_string() });
        }
        
        if self.name.trim().is_empty() {
            return Err(AssetError::ValidationError { message: "Consent category name cannot be empty".to_string() });
        }
        
        for sub_category in &self.sub_categories {
            sub_category.validate()?;
        }
        
        Ok(())
    }
}

impl ConsentWithdrawalSettings {
    pub fn validate(&self) -> AssetResult<()> {
        if self.withdrawal_methods.is_empty() {
            return Err(AssetError::ValidationError { message: "At least one withdrawal method must be specified".to_string() });
        }
        Ok(())
    }
}

impl ConsentVerificationSettings {
    pub fn validate(&self) -> AssetResult<()> {
        if self.verification_frequency.as_secs() == 0 {
            return Err(AssetError::ValidationError { message: "Verification frequency cannot be zero".to_string() });
        }
        Ok(())
    }
}

impl Default for ConsentManagementSettings {
    fn default() -> Self {
        Self {
            consent_model: ConsentModel::Explicit,
            granular_consent: GranularConsentSettings::default(),
            withdrawal_settings: ConsentWithdrawalSettings::default(),
            verification_settings: ConsentVerificationSettings::default(),
            audit_trail: ConsentAuditTrailSettings::default(),
        }
    }
}

impl Default for GranularConsentSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            categories: Vec::new(),
            dependencies: HashMap::new(),
            default_states: HashMap::new(),
        }
    }
}

impl Default for ConsentWithdrawalSettings {
    fn default() -> Self {
        Self {
            withdrawal_methods: vec![WithdrawalMethod::SelfService],
            confirmation_settings: WithdrawalConfirmationSettings::default(),
            grace_period_settings: WithdrawalGracePeriodSettings::default(),
            processing_time_limits: HashMap::new(),
        }
    }
}

impl Default for WithdrawalConfirmationSettings {
    fn default() -> Self {
        Self {
            require_confirmation: true,
            confirmation_methods: vec!["email".to_string()],
            confirmation_timeout: Duration::from_secs(24 * 3600), // 24 hours
            multiple_confirmations: false,
        }
    }
}

impl Default for WithdrawalGracePeriodSettings {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(7 * 24 * 3600), // 7 days
            notifications: Vec::new(),
            allow_cancellation: true,
            automatic_processing: true,
        }
    }
}

impl Default for ConsentVerificationSettings {
    fn default() -> Self {
        Self {
            requirements: ConsentVerificationRequirements::default(),
            reverification: ConsentReverificationSettings::default(),
            verification_methods: vec!["identity_check".to_string()],
            verification_frequency: Duration::from_secs(365 * 24 * 3600), // 1 year
        }
    }
}

impl Default for ConsentVerificationRequirements {
    fn default() -> Self {
        Self {
            identity_verification: false,
            age_verification: false,
            capacity_verification: false,
            documentation_requirements: Vec::new(),
        }
    }
}

impl Default for ConsentReverificationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            triggers: Vec::new(),
            frequency: Duration::from_secs(365 * 24 * 3600), // 1 year
            grace_period: Duration::from_secs(30 * 24 * 3600), // 30 days
        }
    }
}

impl Default for ConsentAuditTrailSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            tracked_events: vec![
                ConsentAuditEvent::ConsentGranted,
                ConsentAuditEvent::ConsentWithdrawn,
                ConsentAuditEvent::ConsentModified,
            ],
            retention_period: Duration::from_secs(7 * 365 * 24 * 3600), // 7 years
            security_settings: HashMap::new(),
            realtime_alerts: false,
        }
    }
}