//! Data retention and deletion management
//!
//! Settings for how long data is kept and when it's deleted.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Auto-deletion settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoDeletionSettings {
    /// Enable automatic deletion based on criteria
    pub enabled: bool,
    
    /// Deletion criteria to apply
    pub deletion_criteria: Vec<DeletionCriterion>,
    
    /// Confirmation settings for deletion
    pub confirmation_settings: DeletionConfirmationSettings,
    
    /// Secure deletion method
    pub secure_deletion_method: SecureDeletionMethod,
}

/// Deletion criterion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeletionCriterion {
    /// Type of criterion
    pub criterion_type: DeletionCriterionType,
    
    /// Threshold value
    pub threshold: String,
    
    /// Whether this criterion is active
    pub active: bool,
    
    /// Resources this criterion applies to
    pub applicable_resources: Vec<String>,
}

/// Types of deletion criteria
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeletionCriterionType {
    /// Delete after specific time period
    Age(Duration),
    /// Delete when file size exceeds limit
    FileSize(u64),
    /// Delete when total storage exceeds limit
    TotalStorage(u64),
    /// Delete when not accessed for period
    LastAccess(Duration),
    /// Delete based on user-defined rules
    Custom(String),
}

/// Deletion confirmation settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeletionConfirmationSettings {
    /// Require confirmation before deletion
    pub require_confirmation: bool,
    
    /// Warning period before deletion
    pub warning_period: Duration,
    
    /// Number of confirmation prompts
    pub confirmation_prompts: u32,
}

/// Secure deletion methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SecureDeletionMethod {
    /// Simple file system deletion
    Standard,
    /// Single overwrite with random data
    SingleOverwrite,
    /// Multiple overwrites (DoD 5220.22-M standard)
    MultipleOverwrite,
    /// Cryptographic shredding
    CryptographicShredding,
}

/// Archive preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchivePreferences {
    /// Enable automatic archiving
    pub enabled: bool,
    
    /// Archive after this duration
    pub archive_after: Duration,
    
    /// Archive location preference
    pub archive_location: ArchiveLocation,
    
    /// Archive encryption settings
    pub encryption_settings: ArchiveEncryptionSettings,
}

/// Archive location options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ArchiveLocation {
    /// Local encrypted storage
    Local,
    /// Remote encrypted storage
    Remote,
    /// Distributed storage across network
    Distributed,
    /// User-specified location
    Custom(String),
}

/// Archive encryption settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchiveEncryptionSettings {
    /// Enable encryption for archives
    pub enabled: bool,
    
    /// Encryption algorithm to use
    pub algorithm: String,
    
    /// Key derivation settings
    pub key_derivation: KeyDerivationSettings,
    
    /// Compression before encryption
    pub compress_before_encrypt: bool,
}

/// Key derivation settings for archives
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyDerivationSettings {
    /// Key derivation method
    pub method: String,
    
    /// Number of iterations (for PBKDF2, etc.)
    pub iterations: u32,
    
    /// Salt length in bytes
    pub salt_length: u32,
}