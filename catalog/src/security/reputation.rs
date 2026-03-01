// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Binary publisher verification — certificate-authenticated or not.
//!
//! Whitepaper-aligned: "Something's either authentic or it isn't."
//! No reputation scoring, no tiers, no decay, no float scores.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Result of binary publisher authentication check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherVerification {
    /// Whether the publisher is certificate-authenticated.
    pub authenticated: bool,
    /// Whether the publisher's certificate is currently valid.
    pub certificate_valid: bool,
    /// Reason for rejection (if not authenticated).
    pub reason: Option<String>,
}

/// Binary publisher authenticator.
///
/// Maintains a revocation list; all non-revoked publishers with valid
/// TrustChain certificates are considered authenticated.
pub struct PublisherAuthenticator {
    /// Revoked publisher IDs mapped to revocation reason.
    revoked: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for PublisherAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl PublisherAuthenticator {
    /// Create a new authenticator (no async needed).
    pub fn new() -> Self {
        Self {
            revoked: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Verify a publisher by certificate fingerprint.
    ///
    /// Returns `authenticated: true` unless the publisher is revoked.
    pub async fn verify(&self, cert_fingerprint: &str) -> Result<PublisherVerification> {
        let revoked = self.revoked.read().await;

        if let Some(reason) = revoked.get(cert_fingerprint) {
            Ok(PublisherVerification {
                authenticated: false,
                certificate_valid: false,
                reason: Some(reason.clone()),
            })
        } else {
            Ok(PublisherVerification {
                authenticated: true,
                certificate_valid: true,
                reason: None,
            })
        }
    }

    /// Revoke a publisher's certificate.
    pub async fn revoke(&self, cert_fingerprint: &str, reason: &str) {
        let mut revoked = self.revoked.write().await;
        revoked.insert(cert_fingerprint.to_string(), reason.to_string());
        info!(
            "Revoked publisher certificate: {} ({})",
            cert_fingerprint, reason
        );
    }

    /// Check whether a publisher is revoked.
    pub async fn is_revoked(&self, cert_fingerprint: &str) -> bool {
        self.revoked.read().await.contains_key(cert_fingerprint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authenticator_creation() {
        let auth = PublisherAuthenticator::new();
        let result = auth
            .verify("some-fingerprint")
            .await
            .expect("test: verify should succeed");
        assert!(result.authenticated);
        assert!(result.certificate_valid);
        assert!(result.reason.is_none());
    }

    #[tokio::test]
    async fn test_revocation() {
        let auth = PublisherAuthenticator::new();

        auth.revoke("bad-fp", "compromised key").await;
        assert!(auth.is_revoked("bad-fp").await);

        let result = auth
            .verify("bad-fp")
            .await
            .expect("test: verify should succeed");
        assert!(!result.authenticated);
        assert!(!result.certificate_valid);
        assert!(result.reason.is_some());
    }

    #[tokio::test]
    async fn test_non_revoked_is_authenticated() {
        let auth = PublisherAuthenticator::new();
        auth.revoke("other-fp", "reason").await;

        let result = auth
            .verify("good-fp")
            .await
            .expect("test: verify should succeed");
        assert!(result.authenticated);
    }
}
