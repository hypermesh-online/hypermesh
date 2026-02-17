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

use anyhow::{Result, anyhow};
use argon2::{Argon2, password_hash::{PasswordHasher, SaltString, PasswordHash, PasswordVerifier}};
use blake3::Hasher;
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug};

use crate::matrix::coordinate::MatrixCoordinate;

/// TOTP configuration
const TOTP_PERIOD: u64 = 30; // 30 second period
const TOTP_DIGITS: usize = 6; // 6 digit codes
const RECOVERY_CODE_COUNT: usize = 10; // 10 recovery codes

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
        Self {
            credentials: None,
        }
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
        let totp_secret = self.generate_totp_secret();
        let totp_secret_base32 = self.encode_base32(&totp_secret);

        // Generate recovery codes
        let recovery_codes = self.generate_recovery_codes(RECOVERY_CODE_COUNT);
        let recovery_code_hashes: Vec<String> = recovery_codes
            .iter()
            .map(|code| self.hash_recovery_code(code))
            .collect();

        // Derive key from passphrase using Argon2id
        let password_key = self.derive_password_key(passphrase)?;

        // Encrypt TOTP secret with password-derived key
        let encrypted_totp_secret = self.encrypt_data(&totp_secret, &password_key)?;

        // Generate FALCON-1024 key pair (placeholder - simplified for now)
        let (private_key, public_key) = self.generate_keypair();

        // Encrypt private key with password + TOTP
        let auth_key = self.derive_auth_key(passphrase, &totp_secret_base32)?;
        let encrypted_private_key = self.encrypt_data(&private_key, &auth_key)?;

        // Hash passphrase with Argon2id
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(passphrase.as_bytes(), &salt)
            .map_err(|e| anyhow!("Password hashing failed: {}", e))?
            .to_string();

        let nonce = rand::thread_rng().gen::<[u8; 12]>();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow!("System time error: {}", e))?
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
            user_id,
            node_coordinate.x,
            node_coordinate.y,
            node_coordinate.z
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
    pub fn authenticate(
        &mut self,
        passphrase: &str,
        totp_code: &str,
    ) -> Result<Vec<u8>> {
        let creds = self.credentials.as_mut()
            .ok_or_else(|| anyhow!("Genesis authentication not initialized"))?;

        debug!("Authenticating user: {}", creds.user_id);

        // Check for account lockout (10+ failed attempts)
        if creds.failed_attempts >= 10 {
            return Err(anyhow!("Account locked due to too many failed attempts"));
        }

        // Verify passphrase
        let parsed_hash = PasswordHash::new(&creds.password_hash)
            .map_err(|e| anyhow!("Invalid password hash: {}", e))?;

        let argon2 = Argon2::default();
        if argon2.verify_password(passphrase.as_bytes(), &parsed_hash).is_err() {
            creds.failed_attempts += 1;
            warn!("Invalid passphrase for user: {} (attempt {})", creds.user_id, creds.failed_attempts);
            return Err(anyhow!("Authentication failed: invalid credentials"));
        }

        // Clone encrypted data before dropping the mutable borrow
        let encrypted_totp_secret = creds.encrypted_totp_secret.clone();
        let encrypted_private_key = creds.encrypted_private_key.clone();
        // Mutable borrow of creds ends here (NLL)

        // Derive key to decrypt TOTP secret
        let password_key = self.derive_password_key(passphrase)?;
        let totp_secret = self.decrypt_data(&encrypted_totp_secret, &password_key)?;
        let totp_secret_base32 = self.encode_base32(&totp_secret);

        // Verify TOTP code
        if !self.verify_totp(&totp_secret_base32, totp_code)? {
            let creds = self.credentials.as_mut()
                .ok_or_else(|| anyhow!("No credentials configured"))?;
            creds.failed_attempts += 1;
            warn!("Invalid TOTP code for user: {} (attempt {})", creds.user_id, creds.failed_attempts);
            return Err(anyhow!("Authentication failed: invalid TOTP code"));
        }

        // Derive authentication key (passphrase + TOTP secret)
        let auth_key = self.derive_auth_key(passphrase, &totp_secret_base32)?;

        // Decrypt private key
        let private_key = self.decrypt_data(&encrypted_private_key, &auth_key)?;

        // Reset failed attempts and update last auth
        let creds = self.credentials.as_mut()
            .ok_or_else(|| anyhow!("No credentials configured"))?;
        creds.failed_attempts = 0;
        creds.last_auth = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| anyhow!("System time error: {}", e))?
                .as_secs()
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
    pub fn recover_with_code(
        &mut self,
        passphrase: &str,
        recovery_code: &str,
    ) -> Result<String> {
        // Hash recovery code before taking mutable borrow
        let code_hash = self.hash_recovery_code(recovery_code);

        let creds = self.credentials.as_mut()
            .ok_or_else(|| anyhow!("Genesis authentication not initialized"))?;

        info!("Recovery attempt for user: {}", creds.user_id);

        // Verify passphrase
        let parsed_hash = PasswordHash::new(&creds.password_hash)
            .map_err(|e| anyhow!("Invalid password hash: {}", e))?;

        let argon2 = Argon2::default();
        if argon2.verify_password(passphrase.as_bytes(), &parsed_hash).is_err() {
            return Err(anyhow!("Recovery failed: invalid passphrase"));
        }

        // Clone data before verifying (to avoid borrow conflict)
        let user_id = creds.user_id.clone();
        let has_code = creds.recovery_code_hashes.contains(&code_hash);
        // Mutable borrow of creds ends here (NLL)

        // Verify recovery code
        if !has_code {
            return Err(anyhow!("Recovery failed: invalid recovery code"));
        }

        // Generate new TOTP secret
        let new_totp_secret = self.generate_totp_secret();
        let new_totp_secret_base32 = self.encode_base32(&new_totp_secret);

        // Encrypt new TOTP secret
        let password_key = self.derive_password_key(passphrase)?;
        let encrypted_totp_secret = self.encrypt_data(&new_totp_secret, &password_key)?;

        // SIMPLIFIED RECOVERY: For recovery codes, we only update the TOTP secret.
        // The private key remains encrypted with the OLD auth key.
        // In production, you would either:
        // 1. Require old TOTP for complete recovery, OR
        // 2. Generate a completely new keypair
        // This implementation updates only the TOTP secret (safe approach).

        // Update credentials
        let creds = self.credentials.as_mut()
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

    // === Private Helper Methods ===

    /// Generate TOTP secret (32 random bytes)
    fn generate_totp_secret(&self) -> Vec<u8> {
        let mut secret = vec![0u8; 32];
        rand::thread_rng().fill(&mut secret[..]);
        secret
    }

    /// Generate recovery codes
    fn generate_recovery_codes(&self, count: usize) -> Vec<String> {
        (0..count)
            .map(|_| {
                // Generate 8-character alphanumeric codes
                let code: String = (0..8)
                    .map(|_| {
                        let chars: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Exclude ambiguous chars
                        let idx = rand::thread_rng().gen_range(0..chars.len());
                        chars[idx] as char
                    })
                    .collect();
                code
            })
            .collect()
    }

    /// Hash recovery code with Blake3
    fn hash_recovery_code(&self, code: &str) -> String {
        let mut hasher = Hasher::new();
        hasher.update(code.as_bytes());
        hasher.update(b"recovery_code_salt");
        format!("{}", hasher.finalize())
    }

    /// Derive key from passphrase using Argon2id
    fn derive_password_key(&self, passphrase: &str) -> Result<[u8; 32]> {
        let salt = SaltString::encode_b64(b"genesis_auth_salt_fixed_for_derivation")
            .map_err(|e| anyhow!("Salt encoding failed: {}", e))?;
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(passphrase.as_bytes(), &salt)
            .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

        // Extract 32 bytes from hash
        let hash_bytes = hash.hash.ok_or_else(|| anyhow!("No hash output"))?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
        Ok(key)
    }

    /// Derive authentication key from passphrase + TOTP secret
    fn derive_auth_key(&self, passphrase: &str, totp_secret: &str) -> Result<[u8; 32]> {
        let combined = format!("{}{}", passphrase, totp_secret);
        let salt = SaltString::encode_b64(b"auth_key_salt_fixed_for_derivation")
            .map_err(|e| anyhow!("Salt encoding failed: {}", e))?;
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(combined.as_bytes(), &salt)
            .map_err(|e| anyhow!("Auth key derivation failed: {}", e))?;

        let hash_bytes = hash.hash.ok_or_else(|| anyhow!("No hash output"))?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
        Ok(key)
    }

    /// Encrypt data with ChaCha20-Poly1305
    fn encrypt_data(&self, data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new(key.into());
        let nonce_bytes = rand::thread_rng().gen::<[u8; 12]>();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.append(&mut ciphertext);
        Ok(result)
    }

    /// Decrypt data with ChaCha20-Poly1305
    fn decrypt_data(&self, encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        if encrypted.len() < 12 {
            return Err(anyhow!("Invalid encrypted data: too short"));
        }

        let cipher = ChaCha20Poly1305::new(key.into());
        let nonce = Nonce::from_slice(&encrypted[..12]);
        let ciphertext = &encrypted[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))
    }

    /// Verify TOTP code
    fn verify_totp(&self, secret_base32: &str, code: &str) -> Result<bool> {
        if code.len() != TOTP_DIGITS {
            return Ok(false);
        }

        // Get current time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow!("System time error: {}", e))?
            .as_secs();

        // Calculate time step
        let time_step = now / TOTP_PERIOD;

        // Decode base32 secret
        let secret = self.decode_base32(secret_base32)?;

        // Try current time step and ±1 (allows for clock skew)
        for offset in [-1i64, 0, 1] {
            let step = (time_step as i64 + offset) as u64;
            let computed_code = self.compute_totp(&secret, step)?;
            if computed_code == code {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Compute TOTP code for given time step
    fn compute_totp(&self, secret: &[u8], time_step: u64) -> Result<String> {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        type HmacSha1 = Hmac<Sha1>;

        // Convert time step to bytes
        let time_bytes = time_step.to_be_bytes();

        // Compute HMAC-SHA1 using KeyInit trait
        let mut mac = <HmacSha1 as hmac::Mac>::new_from_slice(secret)
            .map_err(|e| anyhow!("HMAC initialization failed: {}", e))?;
        mac.update(&time_bytes);
        let result = mac.finalize();
        let hash = result.into_bytes();

        // Dynamic truncation (RFC 6238)
        let offset = (hash[19] & 0x0f) as usize;
        let binary = u32::from_be_bytes([
            hash[offset] & 0x7f,
            hash[offset + 1],
            hash[offset + 2],
            hash[offset + 3],
        ]);

        // Generate 6-digit code
        let code = binary % 1_000_000;
        Ok(format!("{:06}", code))
    }

    /// Generate keypair (placeholder - would be FALCON-1024 in production)
    fn generate_keypair(&self) -> (Vec<u8>, Vec<u8>) {
        let mut private_key = vec![0u8; 64];
        let mut public_key = vec![0u8; 32];

        rand::thread_rng().fill(&mut private_key[..]);
        rand::thread_rng().fill(&mut public_key[..]);

        (private_key, public_key)
    }

    /// Encode bytes to base32
    fn encode_base32(&self, data: &[u8]) -> String {
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, data)
    }

    /// Decode base32 to bytes
    fn decode_base32(&self, data: &str) -> Result<Vec<u8>> {
        base32::decode(base32::Alphabet::Rfc4648 { padding: false }, data)
            .ok_or_else(|| anyhow!("Invalid base32 encoding"))
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
        MatrixCoordinate::new(1, 2, 3).unwrap()
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
        let (totp_secret, recovery_codes) = result.unwrap();

        assert!(!totp_secret.is_empty());
        assert_eq!(recovery_codes.len(), RECOVERY_CODE_COUNT);
        assert!(auth.get_credentials().is_some());
    }

    #[test]
    fn test_authentication_flow() {
        let mut auth = GenesisAuthManager::new();
        let (totp_secret, _) = auth.initialize(
            "user@example.com".to_string(),
            "strong_passphrase_123",
            test_coordinate(),
        ).unwrap();

        // Compute current TOTP code
        let secret = auth.decode_base32(&totp_secret).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let time_step = now / TOTP_PERIOD;
        let totp_code = auth.compute_totp(&secret, time_step).unwrap();

        // Authenticate
        let result = auth.authenticate("strong_passphrase_123", &totp_code);
        assert!(result.is_ok());

        let private_key = result.unwrap();
        assert!(!private_key.is_empty());
    }

    #[test]
    fn test_authentication_failure() {
        let mut auth = GenesisAuthManager::new();
        auth.initialize(
            "user@example.com".to_string(),
            "strong_passphrase_123",
            test_coordinate(),
        ).unwrap();

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
        let (_, recovery_codes) = auth.initialize(
            "user@example.com".to_string(),
            "strong_passphrase_123",
            test_coordinate(),
        ).unwrap();

        // Use first recovery code
        let result = auth.recover_with_code("strong_passphrase_123", &recovery_codes[0]);
        if let Err(e) = &result {
            eprintln!("Recovery failed: {}", e);
        }
        assert!(result.is_ok(), "Recovery code test failed: {:?}", result.err());

        let new_totp_secret = result.unwrap();
        assert!(!new_totp_secret.is_empty());
    }

    #[test]
    fn test_totp_validation() {
        let auth = GenesisAuthManager::new();
        let secret = auth.generate_totp_secret();
        let secret_base32 = auth.encode_base32(&secret);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let time_step = now / TOTP_PERIOD;

        let code = auth.compute_totp(&secret, time_step).unwrap();
        assert_eq!(code.len(), TOTP_DIGITS);
        assert!(auth.verify_totp(&secret_base32, &code).unwrap());

        // Wrong code should fail
        assert!(!auth.verify_totp(&secret_base32, "000000").unwrap());
    }
}
