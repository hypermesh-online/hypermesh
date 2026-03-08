// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! MFA-protected genesis authentication helpers for `NodeBlockchain`.
//!
//! These methods manage the optional multi-factor authentication layer
//! that protects a node's genesis block credentials.

use tracing::info;

use super::chain::NodeBlockchain;
use super::genesis_auth::{GenesisAuthManager, GenesisCredentials};

impl NodeBlockchain {
    /// Initialize MFA-protected genesis authentication.
    ///
    /// # Arguments
    /// * `user_id` - User identifier (username/email)
    /// * `passphrase` - User passphrase for key derivation
    ///
    /// # Returns
    /// Tuple of (TOTP secret for user to save, recovery codes)
    pub async fn initialize_genesis_auth(
        &self,
        user_id: String,
        passphrase: &str,
    ) -> Result<(String, Vec<String>), String> {
        let mut auth_guard = self.genesis_auth.write().await;

        if auth_guard.is_some() {
            return Err("Genesis authentication already initialized".to_string());
        }

        let mut auth_manager = GenesisAuthManager::new();
        let (totp_secret, recovery_codes) = auth_manager
            .initialize(user_id, passphrase, self.node_coordinate)
            .map_err(|e| format!("Failed to initialize genesis auth: {e}"))?;

        *auth_guard = Some(auth_manager);

        info!(
            "Genesis authentication initialized for node ({}, {}, {})",
            self.node_coordinate.x, self.node_coordinate.y, self.node_coordinate.z
        );

        Ok((totp_secret, recovery_codes))
    }

    /// Authenticate and unlock genesis block (MFA required).
    ///
    /// # Arguments
    /// * `passphrase` - User passphrase
    /// * `totp_code` - Current TOTP code (6 digits)
    ///
    /// # Returns
    /// Decrypted private key if authentication successful
    pub async fn authenticate_genesis(
        &self,
        passphrase: &str,
        totp_code: &str,
    ) -> Result<Vec<u8>, String> {
        let mut auth_guard = self.genesis_auth.write().await;

        let auth_manager = auth_guard
            .as_mut()
            .ok_or_else(|| "Genesis authentication not initialized".to_string())?;

        auth_manager
            .authenticate(passphrase, totp_code)
            .map_err(|e| format!("Authentication failed: {e}"))
    }

    /// Recover genesis access using recovery code.
    ///
    /// # Arguments
    /// * `passphrase` - User passphrase
    /// * `recovery_code` - One of the recovery codes
    ///
    /// # Returns
    /// New TOTP secret (user must save this)
    pub async fn recover_genesis(
        &self,
        passphrase: &str,
        recovery_code: &str,
    ) -> Result<String, String> {
        let mut auth_guard = self.genesis_auth.write().await;

        let auth_manager = auth_guard
            .as_mut()
            .ok_or_else(|| "Genesis authentication not initialized".to_string())?;

        auth_manager
            .recover_with_code(passphrase, recovery_code)
            .map_err(|e| format!("Recovery failed: {e}"))
    }

    /// Get genesis credentials for serialization/storage.
    pub async fn get_genesis_credentials(&self) -> Option<GenesisCredentials> {
        let auth_guard = self.genesis_auth.read().await;
        auth_guard
            .as_ref()
            .and_then(|auth| auth.get_credentials().cloned())
    }

    /// Load genesis credentials from external storage.
    pub async fn load_genesis_credentials(
        &self,
        credentials: GenesisCredentials,
    ) -> Result<(), String> {
        let mut auth_guard = self.genesis_auth.write().await;

        if auth_guard.is_some() {
            return Err("Genesis authentication already loaded".to_string());
        }

        let mut auth_manager = GenesisAuthManager::new();
        auth_manager
            .load_credentials(credentials)
            .map_err(|e| format!("Failed to load credentials: {e}"))?;

        *auth_guard = Some(auth_manager);
        Ok(())
    }
}
