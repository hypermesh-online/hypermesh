//! Key management for privacy systems
//!
//! Cryptographic key handling, rotation, and recovery.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Key management settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyManagementSettings {
    /// Key derivation method
    pub key_derivation_method: KeyDerivationMethod,
    
    /// Key rotation settings
    pub key_rotation: KeyRotationSettings,
    
    /// Key recovery settings
    pub key_recovery: KeyRecoverySettings,
}

/// Key derivation methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KeyDerivationMethod {
    /// PBKDF2 with SHA-256
    PBKDF2SHA256 { iterations: u32 },
    /// Argon2id (recommended)
    Argon2id { memory_usage: u32, iterations: u32, parallelism: u32 },
    /// scrypt
    Scrypt { n: u32, r: u32, p: u32 },
    /// Hardware-based derivation
    Hardware,
}

/// Key rotation settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRotationSettings {
    /// Enable automatic key rotation
    pub enabled: bool,
    
    /// Rotation interval
    pub rotation_interval: Duration,
    
    /// Key rotation method
    pub rotation_method: KeyRotationMethod,
    
    /// Number of historical keys to keep
    pub historical_keys_count: u32,
    
    /// Grace period for old keys
    pub grace_period: Duration,
}

/// Key rotation methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KeyRotationMethod {
    /// Time-based rotation
    TimeBased,
    /// Usage-based rotation (after N operations)
    UsageBased(u64),
    /// Manual rotation only
    Manual,
}

/// Key recovery settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRecoverySettings {
    /// Enable key recovery
    pub enabled: bool,
    
    /// Recovery method
    pub recovery_method: KeyRecoveryMethod,
    
    /// Recovery limitations
    pub limitations: KeyRecoveryLimitations,
}

/// Key recovery methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum KeyRecoveryMethod {
    /// Secret sharing (Shamir's scheme)
    SecretSharing { threshold: u32, total_shares: u32 },
    /// Recovery phrase (BIP39 mnemonic)
    RecoveryPhrase { word_count: u32 },
    /// Hardware-based recovery
    Hardware,
    /// Social recovery with trusted contacts
    SocialRecovery { required_approvals: u32 },
}

/// Key recovery limitations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRecoveryLimitations {
    /// Maximum recovery attempts
    pub max_attempts: u32,
    
    /// Cooldown period between attempts
    pub attempt_cooldown: Duration,
    
    /// Recovery window (time limit for recovery)
    pub recovery_window: Option<Duration>,
    
    /// Require additional authentication
    pub require_additional_auth: bool,
}