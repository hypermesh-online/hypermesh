// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate management for STOQ transport with TrustChain integration
//!
//! This module provides certificate management for STOQ nodes with:
//! - TrustChain CA integration for production certificates
//! - Self-signed certificates for localhost testing only
//! - Automatic 24-hour certificate rotation
//! - Real-time certificate fingerprinting and validation
//! - Generic validation hooks for application-specific logic

pub mod manager;
pub mod trustchain_client;
pub mod types;

// Re-export public API so external callers remain unchanged
pub use manager::CertificateManager;
pub use trustchain_client::TrustChainClient;
pub use types::{CertificateConfig, CertificateMode, StoqNodeCertificate};

// Re-export parent module items needed by submodules
use super::certificate_strategy;

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_localhost_certificate_manager() -> Result<()> {
        let config = CertificateConfig::default();
        let manager = CertificateManager::new(config).await?;

        let _crypto_config = manager.server_crypto_config().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_certificate_fingerprint() -> Result<()> {
        let config = CertificateConfig::default();
        let manager = CertificateManager::new(config).await?;

        let fingerprint = manager.get_certificate_fingerprint().await?;
        assert!(!fingerprint.is_empty());
        assert_eq!(fingerprint.len(), 64); // SHA-256 hex string
        Ok(())
    }

    #[tokio::test]
    async fn test_certificate_rotation_check() -> Result<()> {
        let config = CertificateConfig::default();
        let manager = CertificateManager::new(config).await?;

        let needs_rotation = manager.check_and_rotate_certificate().await?;
        // Should not need rotation immediately after creation
        assert!(!needs_rotation);
        Ok(())
    }

    #[tokio::test]
    async fn test_real_private_key_generation() -> Result<()> {
        let client = TrustChainClient::new("test".to_string(), "test-node".to_string());
        let private_key = client.generate_real_private_key()?;

        // Verify that private key is not empty and has reasonable size
        assert!(!private_key.is_empty());
        assert!(private_key.len() > 100); // PKCS#8 DER should be substantial
        Ok(())
    }
}
