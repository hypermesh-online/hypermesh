//! Security Configuration
//!
//! Security-related privacy configuration including encryption, authentication,
//! and audit requirements.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetResult, AssetError};

// Re-export from data_management
pub use super::data_management::{
    ArchiveEncryptionSettings, KeyManagementSettings, KeyDerivationMethod,
    KeyRotationSettings, KeyRotationMethod, KeyRecoverySettings, KeyRecoveryMethod,
    KeyRecoveryLimitations
};

/// Security requirements for delivery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliverySecurityRequirements {
    /// Encryption requirements
    pub encryption: EncryptionRequirement,
    
    /// Authentication requirements
    pub authentication: AuthenticationRequirement,
    
    /// Certificate requirements
    pub certificates: CertificateRequirements,
    
    /// Access control requirements
    pub access_control: AccessControlRequirement,
    
    /// Audit trail requirements
    pub audit_trail: AuditTrailRequirement,
}

/// Encryption requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptionRequirement {
    /// Required encryption algorithms
    pub required_algorithms: Vec<String>,
    
    /// Minimum key length
    pub min_key_length: u32,
    
    /// End-to-end encryption required
    pub end_to_end_required: bool,
    
    /// In-transit encryption required
    pub in_transit_required: bool,
    
    /// At-rest encryption required
    pub at_rest_required: bool,
}

/// Authentication requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticationRequirement {
    /// Multi-factor authentication required
    pub mfa_required: bool,
    
    /// Allowed authentication methods
    pub allowed_methods: Vec<String>,
    
    /// Session timeout
    pub session_timeout: Duration,
    
    /// Re-authentication frequency
    pub reauth_frequency: Duration,
}

/// Certificate requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateRequirements {
    /// Client certificates required
    pub client_cert_required: bool,
    
    /// Server certificates required
    pub server_cert_required: bool,
    
    /// Certificate authorities
    pub trusted_cas: Vec<String>,
    
    /// Certificate validation level
    pub validation_level: String,
}

/// Access control requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessControlRequirement {
    /// Authorization model
    pub authorization_model: AuthorizationModel,
    
    /// Role-based access control
    pub rbac_enabled: bool,
    
    /// Attribute-based access control
    pub abac_enabled: bool,
    
    /// Fine-grained permissions
    pub fine_grained_permissions: bool,
}

/// Authorization models
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthorizationModel {
    RoleBased,
    AttributeBased,
    PolicyBased,
    Hybrid,
}

/// Audit trail requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditTrailRequirement {
    /// Audit logging enabled
    pub enabled: bool,
    
    /// Audit detail level
    pub detail_level: String,
    
    /// Audit retention period
    pub retention_period: Duration,
    
    /// Immutable audit logs
    pub immutable_logs: bool,
    
    /// Real-time audit alerts
    pub realtime_alerts: bool,
}

impl Default for DeliverySecurityRequirements {
    fn default() -> Self {
        Self {
            encryption: EncryptionRequirement::default(),
            authentication: AuthenticationRequirement::default(),
            certificates: CertificateRequirements::default(),
            access_control: AccessControlRequirement::default(),
            audit_trail: AuditTrailRequirement::default(),
        }
    }
}

impl Default for EncryptionRequirement {
    fn default() -> Self {
        Self {
            required_algorithms: vec!["AES-256-GCM".to_string()],
            min_key_length: 256,
            end_to_end_required: true,
            in_transit_required: true,
            at_rest_required: true,
        }
    }
}

impl Default for AuthenticationRequirement {
    fn default() -> Self {
        Self {
            mfa_required: false,
            allowed_methods: vec!["password".to_string(), "token".to_string()],
            session_timeout: Duration::from_secs(3600), // 1 hour
            reauth_frequency: Duration::from_secs(24 * 3600), // 24 hours
        }
    }
}

impl Default for CertificateRequirements {
    fn default() -> Self {
        Self {
            client_cert_required: false,
            server_cert_required: true,
            trusted_cas: Vec::new(),
            validation_level: "standard".to_string(),
        }
    }
}

impl Default for AccessControlRequirement {
    fn default() -> Self {
        Self {
            authorization_model: AuthorizationModel::RoleBased,
            rbac_enabled: true,
            abac_enabled: false,
            fine_grained_permissions: false,
        }
    }
}

impl Default for AuditTrailRequirement {
    fn default() -> Self {
        Self {
            enabled: true,
            detail_level: "standard".to_string(),
            retention_period: Duration::from_secs(365 * 24 * 3600), // 1 year
            immutable_logs: true,
            realtime_alerts: false,
        }
    }
}