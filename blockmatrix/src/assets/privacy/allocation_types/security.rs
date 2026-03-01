// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security requirements for privacy allocation types

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Security requirements for allocation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityRequirements {
    /// Encryption requirements
    pub encryption_requirements: EncryptionRequirements,

    /// Authentication requirements
    pub authentication_requirements: AuthenticationRequirements,

    /// Audit logging requirements
    pub audit_requirements: AuditRequirements,

    /// Data protection requirements
    pub data_protection: DataProtectionRequirements,
}

/// Encryption requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptionRequirements {
    /// Require encryption in transit
    pub require_transport_encryption: bool,

    /// Require encryption at rest
    pub require_storage_encryption: bool,

    /// Minimum encryption strength
    pub minimum_key_length: u32,

    /// Allowed encryption algorithms
    pub allowed_algorithms: Vec<String>,

    /// Require quantum-resistant encryption
    pub require_quantum_resistant: bool,
}

/// Authentication requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticationRequirements {
    /// Authentication methods required
    pub required_methods: Vec<AuthenticationMethod>,

    /// Multi-factor authentication required
    pub require_mfa: bool,

    /// Certificate validation required
    pub require_certificate_validation: bool,

    /// Biometric authentication required
    pub require_biometric: bool,

    /// Session management requirements
    pub session_requirements: SessionRequirements,
}

/// Authentication method types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    Password,
    Certificate,
    Token,
    Biometric,
    Hardware,
    ConsensusProof,
}

/// Session management requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionRequirements {
    /// Maximum session duration
    pub max_session_duration: Duration,

    /// Session idle timeout
    pub idle_timeout: Duration,

    /// Require session renewal
    pub require_renewal: bool,

    /// Session binding requirements
    pub binding_requirements: Vec<String>,
}

/// Audit logging requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditRequirements {
    /// Events that must be logged
    pub required_events: Vec<String>,

    /// Log retention period
    pub retention_period: Duration,

    /// Real-time monitoring required
    pub require_realtime_monitoring: bool,

    /// External audit system integration
    pub external_audit_integration: bool,
}

/// Data protection requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataProtectionRequirements {
    /// Data classification requirements
    pub classification_requirements: Vec<String>,

    /// Data retention policies
    pub retention_policies: Vec<RetentionPolicy>,

    /// Data anonymization requirements
    pub anonymization_requirements: AnonymizationRequirements,

    /// Cross-border transfer restrictions
    pub transfer_restrictions: Vec<String>,
}

/// Data retention policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Data type
    pub data_type: String,

    /// Retention period
    pub retention_period: Duration,

    /// Automatic deletion
    pub auto_delete: bool,

    /// Archive policy
    pub archive_policy: Option<ArchivePolicy>,
}

/// Archive policy for retained data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchivePolicy {
    /// Archive after period
    pub archive_after: Duration,

    /// Archive location
    pub archive_location: String,

    /// Archive encryption
    pub archive_encryption: bool,
}

/// Data anonymization requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymizationRequirements {
    /// Anonymization techniques required
    pub required_techniques: Vec<AnonymizationTechnique>,

    /// K-anonymity level
    pub k_anonymity_level: Option<u32>,

    /// Differential privacy parameters
    pub differential_privacy: Option<DifferentialPrivacyParams>,
}

/// Data anonymization techniques
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnonymizationTechnique {
    Hashing,
    Tokenization,
    Generalization,
    Suppression,
    Noise,
    DifferentialPrivacy,
}

/// Differential privacy parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DifferentialPrivacyParams {
    /// Privacy budget (epsilon)
    pub epsilon: f32,

    /// Delta parameter
    pub delta: f32,

    /// Sensitivity
    pub sensitivity: f32,
}
