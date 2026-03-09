// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Genesis Block User Authentication with MFA
//!
//! Implements user-specific authentication for genesis block creation:
//! - MFA-based lock/unlock mechanism (TOTP authentication)
//! - User passphrase + TOTP token for key encryption
//! - Recovery codes for account recovery
//! - Matrix coordinate integration
//! - Self-authentication without external CA
//!
//! Cryptographic primitives (TOTP, encrypt/decrypt, key derivation) live in
//! the sibling `genesis_crypto` module.

use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chacha20poly1305::aead::{KeyInit, OsRng};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use super::genesis_crypto;
use crate::matrix::coordinate::MatrixCoordinate;

/// User authentication credentials for genesis block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisCredentials {
    /// User identifier (username/email)
    pub user_id: String,

    /// Argon2id password hash
    pub password_hash: String,

    /// TOTP secret (encrypted with password-derived key)
    pub encrypted_totp_secret: Vec<u8>,

    /// Recovery codes (hashed with Blake3)
    pub recovery_code_hashes: Vec<String>,

    /// Matrix coordinate for this node
    pub node_coordinate: MatrixCoordinate,

    /// Encrypted private key (ChaCha20-Poly1305)
    pub encrypted_private_key: Vec<u8>,

    /// Public key (FALCON-1024 placeholder - would be real quantum-resistant key)
    pub public_key: Vec<u8>,

    /// Nonce for encryption
    pub nonce: [u8; 12],

    /// Creation timestamp
    pub created_at: u64,

    /// Last authentication timestamp
    pub last_auth: Option<u64>,

    /// Failed authentication attempts
    pub failed_attempts: u32,
}

/// Genesis authentication manager
pub struct GenesisAuthManager {
    /// Current credentials (None if not initialized)
    credentials: Option<GenesisCredentials>,
}

impl GenesisAuthManager {
    /// Create new authentication manager
    pub fn new() -> Self {
        Self { credentials: None }
    }

    /// Initialize genesis authentication with user credentials
    ///
    /// # Arguments
    /// * `user_id` - User identifier (username/email)
    /// * `passphrase` - User passphrase for key derivation
    /// * `node_coordinate` - Matrix coordinate for this node
    ///
    /// # Returns
    /// Tuple of (TOTP secret for user to save, recovery codes)
    pub fn initialize(
        &mut self,
        user_id: String,
        passphrase: &str,
        node_coordinate: MatrixCoordinate,
    ) -> Result<(String, Vec<String>)> {
        if self.credentials.is_some() {
            return Err(anyhow!("Genesis authentication already initialized"));
        }

        info!("Initializing genesis authentication for user: {}", user_id);

        // Generate TOTP secret (32 random bytes, base32 encoded)
        let totp_secret = genesis_crypto::generate_totp_secret();
        let totp_secret_base32 = genesis_crypto::encode_base32(&totp_secret);

        // Generate recovery codes
        let recovery_codes =
            genesis_crypto::generate_recovery_codes(genesis_crypto::RECOVERY_CODE_COUNT);
        let recovery_code_hashes: Vec<String> = recovery_codes
            .iter()
            .map(|code| genesis_crypto::hash_recovery_code(code))
            .collect();

        // Derive key from passphrase using Argon2id
        let password_key = genesis_crypto::derive_password_key(passphrase)?;

        // Encrypt TOTP secret with password-derived key
        let encrypted_totp_secret = genesis_crypto::encrypt_data(&totp_secret, &password_key)?;

        // Generate FALCON-1024 key pair (placeholder - simplified for now)
        let (private_key, public_key) = genesis_crypto::generate_keypair();

        // Encrypt private key with password + TOTP
        let auth_key = genesis_crypto::derive_auth_key(passphrase, &totp_secret_base32)?;
        let encrypted_private_key = genesis_crypto::encrypt_data(&private_key, &auth_key)?;

        // Hash passphrase with Argon2id
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(passphrase.as_bytes(), &salt)
            .map_err(|e| anyhow!("Password hashing failed: {e}"))?
            .to_string();

        let nonce = rand::thread_rng().gen::<[u8; 12]>();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow!("System time error: {e}"))?
            .as_secs();

        self.credentials = Some(GenesisCredentials {
            user_id: user_id.clone(),
            password_hash,
            encrypted_totp_secret,
            recovery_code_hashes,
            node_coordinate,
            encrypted_private_key,
            public_key,
            nonce,
            created_at: now,
            last_auth: None,
            failed_attempts: 0,
        });

        info!(
            "Genesis authentication initialized for {} at matrix ({}, {}, {})",
            user_id, node_coordinate.x, node_coordinate.y, node_coordinate.z
        );

        Ok((totp_secret_base32, recovery_codes))
    }

    /// Authenticate and unlock genesis block
    ///
    /// # Arguments
    /// * `passphrase` - User passphrase
    /// * `totp_code` - Current TOTP code (6 digits)
    ///
    /// # Returns
    /// Decrypted private key if authentication successful
    pub fn authenticate(&mut self, passphrase: &str, totp_code: &str) -> Result<Vec<u8>> {
        let creds = self
            .credentials
            .as_mut()
            .ok_or_else(|| anyhow!("Genesis authentication not initialized"))?;

        // Check for account lockout (10+ failed attempts)
        if creds.failed_attempts >= 10 {
            return Err(anyhow!("Account locked due to too many failed attempts"));
        }

        // Verify passphrase
        let parsed_hash = PasswordHash::new(&creds.password_hash)
            .map_err(|e| anyhow!("Invalid password hash: {e}"))?;

        let argon2 = Argon2::default();
        if argon2
            .verify_password(passphrase.as_bytes(), &parsed_hash)
            .is_err()
        {
            creds.failed_attempts += 1;
            warn!(
                "Invalid passphrase for user: {} (attempt {})",
                creds.user_id, creds.failed_attempts
            );
            return Err(anyhow!("Authentication failed: invalid credentials"));
        }

        // Clone encrypted data before dropping the mutable borrow
        let encrypted_totp_secret = creds.encrypted_totp_secret.clone();
        let encrypted_private_key = creds.encrypted_private_key.clone();

        // Derive key to decrypt TOTP secret
        let password_key = genesis_crypto::derive_password_key(passphrase)?;
        let totp_secret = genesis_crypto::decrypt_data(&encrypted_totp_secret, &password_key)?;
        let totp_secret_base32 = genesis_crypto::encode_base32(&totp_secret);

        // Verify TOTP code
        if !genesis_crypto::verify_totp(&totp_secret_base32, totp_code)? {
            let creds = self
                .credentials
                .as_mut()
                .ok_or_else(|| anyhow!("No credentials configured"))?;
            creds.failed_attempts += 1;
            warn!(
                "Invalid TOTP code for user: {} (attempt {})",
                creds.user_id, creds.failed_attempts
            );
            return Err(anyhow!("Authentication failed: invalid TOTP code"));
        }

        // Derive authentication key (passphrase + TOTP secret)
        let auth_key = genesis_crypto::derive_auth_key(passphrase, &totp_secret_base32)?;

        // Decrypt private key
        let private_key = genesis_crypto::decrypt_data(&encrypted_private_key, &auth_key)?;

        // Reset failed attempts and update last auth
        let creds = self
            .credentials
            .as_mut()
            .ok_or_else(|| anyhow!("No credentials configured"))?;
        creds.failed_attempts = 0;
        creds.last_auth = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| anyhow!("System time error: {e}"))?
                .as_secs(),
        );
        let user_id = creds.user_id.clone();

        info!("Authentication successful for user: {}", user_id);
        Ok(private_key)
    }

    /// Recover access using recovery code
    ///
    /// # Arguments
    /// * `passphrase` - User passphrase
    /// * `recovery_code` - One of the recovery codes
    ///
    /// # Returns
    /// New TOTP secret (user must save this)
    pub fn recover_with_code(&mut self, passphrase: &str, recovery_code: &str) -> Result<String> {
        // Hash recovery code before taking mutable borrow
        let code_hash = genesis_crypto::hash_recovery_code(recovery_code);

        let creds = self
            .credentials
            .as_mut()
            .ok_or_else(|| anyhow!("Genesis authentication not initialized"))?;

        info!("Recovery attempt for user: {}", creds.user_id);

        // Verify passphrase
        let parsed_hash = PasswordHash::new(&creds.password_hash)
            .map_err(|e| anyhow!("Invalid password hash: {e}"))?;

        let argon2 = Argon2::default();
        if argon2
            .verify_password(passphrase.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(anyhow!("Recovery failed: invalid passphrase"));
        }

        // Clone data before verifying (to avoid borrow conflict)
        let user_id = creds.user_id.clone();
        let has_code = creds.recovery_code_hashes.contains(&code_hash);

        // Verify recovery code
        if !has_code {
            return Err(anyhow!("Recovery failed: invalid recovery code"));
        }

        // Generate new TOTP secret
        let new_totp_secret = genesis_crypto::generate_totp_secret();
        let new_totp_secret_base32 = genesis_crypto::encode_base32(&new_totp_secret);

        // Encrypt new TOTP secret
        let password_key = genesis_crypto::derive_password_key(passphrase)?;
        let encrypted_totp_secret =
            genesis_crypto::encrypt_data(&new_totp_secret, &password_key)?;

        // Update credentials
        let creds = self
            .credentials
            .as_mut()
            .ok_or_else(|| anyhow!("No credentials configured"))?;
        creds.encrypted_totp_secret = encrypted_totp_secret;
        creds.failed_attempts = 0;

        info!("Recovery successful for user: {} (TOTP updated)", user_id);
        Ok(new_totp_secret_base32)
    }

    /// Get credentials (for serialization/storage)
    pub fn get_credentials(&self) -> Option<&GenesisCredentials> {
        self.credentials.as_ref()
    }

    /// Load credentials from external storage
    pub fn load_credentials(&mut self, credentials: GenesisCredentials) -> Result<()> {
        if self.credentials.is_some() {
            return Err(anyhow!("Credentials already loaded"));
        }
        self.credentials = Some(credentials);
        Ok(())
    }
}

impl Default for GenesisAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_coordinate() -> MatrixCoordinate {
        MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate")
    }

    #[test]
    fn test_genesis_initialization() {
        let mut auth = GenesisAuthManager::new();
        let result = auth.initialize(
            "user@example.com".to_string(),
            "strong_passphrase_123",
            test_coordinate(),
        );

        assert!(result.is_ok());
        let (totp_secret, recovery_codes) = result.expect("test: expected success");

        assert!(!totp_secret.is_empty());
        assert_eq!(recovery_codes.len(), genesis_crypto::RECOVERY_CODE_COUNT);
        assert!(auth.get_credentials().is_some());
    }

    #[test]
    fn test_authentication_flow() {
        let mut auth = GenesisAuthManager::new();
        let (totp_secret, _) = auth
            .initialize(
                "user@example.com".to_string(),
                "strong_passphrase_123",
                test_coordinate(),
            )
            .expect("test: expected success");

        // Compute current TOTP code
        let secret = genesis_crypto::decode_base32(&totp_secret).expect("test: expected success");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test: expected success")
            .as_secs();
        let time_step = now / genesis_crypto::TOTP_PERIOD;
        let totp_code =
            genesis_crypto::compute_totp(&secret, time_step).expect("test: expected success");

        // Authenticate
        let result = auth.authenticate("strong_passphrase_123", &totp_code);
        assert!(result.is_ok());

        let private_key = result.expect("test: expected success");
        assert!(!private_key.is_empty());
    }

    #[test]
    fn test_authentication_failure() {
        let mut auth = GenesisAuthManager::new();
        auth.initialize(
            "user@example.com".to_string(),
            "strong_passphrase_123",
            test_coordinate(),
        )
        .expect("test: expected success");

        // Wrong passphrase
        let result = auth.authenticate("wrong_passphrase", "123456");
        assert!(result.is_err());

        // Wrong TOTP
        let result = auth.authenticate("strong_passphrase_123", "000000");
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_code() {
        let mut auth = GenesisAuthManager::new();
        let (_, recovery_codes) = auth
            .initialize(
                "user@example.com".to_string(),
                "strong_passphrase_123",
                test_coordinate(),
            )
            .expect("test: expected success");

        // Use first recovery code
        let result = auth.recover_with_code("strong_passphrase_123", &recovery_codes[0]);
        if let Err(e) = &result {
            eprintln!("Recovery failed: {e}");
        }
        assert!(
            result.is_ok(),
            "Recovery code test failed: {:?}",
            result.err()
        );

        let new_totp_secret = result.expect("test: expected success");
        assert!(!new_totp_secret.is_empty());
    }

    #[test]
    fn test_totp_validation() {
        let secret = genesis_crypto::generate_totp_secret();
        let secret_base32 = genesis_crypto::encode_base32(&secret);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test: expected success")
            .as_secs();
        let time_step = now / genesis_crypto::TOTP_PERIOD;

        let code =
            genesis_crypto::compute_totp(&secret, time_step).expect("test: expected success");
        assert_eq!(code.len(), genesis_crypto::TOTP_DIGITS);
        assert!(
            genesis_crypto::verify_totp(&secret_base32, &code).expect("test: assertion value")
        );

        // Wrong code should fail
        assert!(
            !genesis_crypto::verify_totp(&secret_base32, "000000").expect("test: assertion value")
        );
    }
}
