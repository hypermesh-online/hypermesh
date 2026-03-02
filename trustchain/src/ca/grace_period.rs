// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Offline Device Grace Period Renewal
//!
//! When a device comes back online after being offline, its certificate may
//! have expired. The `GracePeriodManager` allows a configurable grace window
//! during which the device can re-enroll without losing its identity.
//!
//! Grace periods are scope-dependent:
//! - Anonymous: no grace period (ephemeral by nature)
//! - Private (Device scope): 7 days
//! - Public (Network scope): 30 days

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use hypermesh_lib::PrivacyMode;

use crate::errors::{Result as TrustChainResult, TrustChainError};

/// Configuration for grace period durations per privacy scope.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GracePeriodConfig {
    /// Grace period for Anonymous scope (should be zero).
    pub anonymous_grace: Duration,
    /// Grace period for Private/bounded scope.
    pub private_grace: Duration,
    /// Grace period for Public/network scope.
    pub public_grace: Duration,
    /// Maximum number of grace renewals per certificate.
    pub max_renewals_per_cert: u32,
}

impl Default for GracePeriodConfig {
    fn default() -> Self {
        Self {
            anonymous_grace: Duration::ZERO,
            private_grace: Duration::from_secs(7 * 24 * 60 * 60),  // 7 days
            public_grace: Duration::from_secs(30 * 24 * 60 * 60),  // 30 days
            max_renewals_per_cert: 3,
        }
    }
}

/// A temporary renewal token issued during the grace period.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenewalToken {
    /// Unique token identifier.
    pub token_id: String,
    /// Serial number of the expired certificate.
    pub original_serial: String,
    /// When the token was issued.
    pub issued_at: SystemTime,
    /// When the token expires (end of grace window).
    pub expires_at: SystemTime,
    /// The privacy scope that determined the grace period.
    pub scope: GraceScope,
}

/// Simplified scope for grace period lookups.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraceScope {
    Anonymous,
    Private,
    Public,
}

impl From<&PrivacyMode> for GraceScope {
    fn from(mode: &PrivacyMode) -> Self {
        if !mode.tracked {
            GraceScope::Anonymous
        } else if mode.scope == hypermesh_lib::AccessScope::Bounded {
            GraceScope::Private
        } else {
            GraceScope::Public
        }
    }
}

/// Tracks grace period usage for each certificate.
#[derive(Clone, Debug, Default)]
struct GraceUsage {
    /// Number of times a grace renewal was used.
    renewal_count: u32,
    /// Token IDs issued for this certificate.
    tokens: Vec<String>,
}

/// Manages grace period renewals for devices that come back online
/// after their certificates have expired.
pub struct GracePeriodManager {
    config: GracePeriodConfig,
    /// Grace usage per certificate serial number.
    usage: Arc<RwLock<HashMap<String, GraceUsage>>>,
    /// Active renewal tokens indexed by token_id.
    active_tokens: Arc<RwLock<HashMap<String, RenewalToken>>>,
}

impl GracePeriodManager {
    /// Create a new grace period manager with default configuration.
    pub fn new() -> Self {
        Self::with_config(GracePeriodConfig::default())
    }

    /// Create a grace period manager with custom configuration.
    pub fn with_config(config: GracePeriodConfig) -> Self {
        Self {
            config,
            usage: Arc::new(RwLock::new(HashMap::new())),
            active_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an expired certificate is within its grace period.
    ///
    /// Returns `true` if the certificate expired recently enough to qualify
    /// for grace period renewal.
    pub fn is_within_grace_period(
        &self,
        expired_at: SystemTime,
        scope: &GraceScope,
    ) -> bool {
        let grace = self.grace_duration(scope);
        if grace.is_zero() {
            return false;
        }

        let now = SystemTime::now();
        let deadline = expired_at + grace;
        now <= deadline
    }

    /// Request a grace period renewal for an expired certificate.
    ///
    /// Returns a `RenewalToken` if the certificate qualifies. Rejects if:
    /// - The scope has no grace period (Anonymous)
    /// - The certificate is past its grace window
    /// - The maximum number of renewals has been exceeded
    pub async fn request_renewal(
        &self,
        serial_number: &str,
        expired_at: SystemTime,
        scope: &GraceScope,
    ) -> TrustChainResult<RenewalToken> {
        // Check scope allows grace period
        let grace = self.grace_duration(scope);
        if grace.is_zero() {
            return Err(TrustChainError::InvalidRequest {
                reason: format!(
                    "No grace period for {:?} scope",
                    scope
                ),
            });
        }

        // Check within grace window
        if !self.is_within_grace_period(expired_at, scope) {
            return Err(TrustChainError::InvalidRequest {
                reason: format!(
                    "Certificate '{}' is past its grace period",
                    serial_number
                ),
            });
        }

        // Check renewal count
        {
            let usage = self.usage.read().await;
            if let Some(u) = usage.get(serial_number) {
                if u.renewal_count >= self.config.max_renewals_per_cert {
                    return Err(TrustChainError::InvalidRequest {
                        reason: format!(
                            "Certificate '{}' has exceeded maximum grace renewals ({})",
                            serial_number, self.config.max_renewals_per_cert
                        ),
                    });
                }
            }
        }

        // Issue renewal token
        let now = SystemTime::now();
        let token = RenewalToken {
            token_id: uuid::Uuid::new_v4().to_string(),
            original_serial: serial_number.to_string(),
            issued_at: now,
            expires_at: expired_at + grace,
            scope: scope.clone(),
        };

        // Record usage
        {
            let mut usage = self.usage.write().await;
            let entry = usage.entry(serial_number.to_string()).or_default();
            entry.renewal_count += 1;
            entry.tokens.push(token.token_id.clone());
        }

        // Store active token
        {
            let mut tokens = self.active_tokens.write().await;
            tokens.insert(token.token_id.clone(), token.clone());
        }

        info!(
            "Grace period renewal token issued for cert '{}' ({:?} scope, renewal #{})",
            serial_number,
            scope,
            self.get_renewal_count(serial_number).await
        );

        Ok(token)
    }

    /// Validate that a renewal token is still active and not expired.
    pub async fn validate_token(&self, token_id: &str) -> TrustChainResult<RenewalToken> {
        let tokens = self.active_tokens.read().await;
        let token = tokens.get(token_id).ok_or_else(|| {
            TrustChainError::InvalidRequest {
                reason: format!("Renewal token '{}' not found", token_id),
            }
        })?;

        let now = SystemTime::now();
        if now > token.expires_at {
            return Err(TrustChainError::InvalidRequest {
                reason: format!("Renewal token '{}' has expired", token_id),
            });
        }

        Ok(token.clone())
    }

    /// Consume a renewal token (marks it as used and removes it).
    pub async fn consume_token(&self, token_id: &str) -> TrustChainResult<RenewalToken> {
        let token = self.validate_token(token_id).await?;
        let mut tokens = self.active_tokens.write().await;
        tokens.remove(token_id);
        debug!("Consumed renewal token '{}'", token_id);
        Ok(token)
    }

    /// Get the number of grace renewals used by a certificate.
    pub async fn get_renewal_count(&self, serial_number: &str) -> u32 {
        self.usage
            .read()
            .await
            .get(serial_number)
            .map(|u| u.renewal_count)
            .unwrap_or(0)
    }

    fn grace_duration(&self, scope: &GraceScope) -> Duration {
        match scope {
            GraceScope::Anonymous => self.config.anonymous_grace,
            GraceScope::Private => self.config.private_grace,
            GraceScope::Public => self.config.public_grace,
        }
    }
}

impl Default for GracePeriodManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recent_expiry() -> SystemTime {
        // Expired 1 hour ago
        SystemTime::now() - Duration::from_secs(3600)
    }

    fn old_expiry() -> SystemTime {
        // Expired 60 days ago
        SystemTime::now() - Duration::from_secs(60 * 24 * 3600)
    }

    #[tokio::test]
    async fn test_private_scope_grace_period() {
        let mgr = GracePeriodManager::new();

        // Recently expired cert in private scope should qualify
        assert!(mgr.is_within_grace_period(recent_expiry(), &GraceScope::Private));

        let token = mgr
            .request_renewal("cert-priv-1", recent_expiry(), &GraceScope::Private)
            .await
            .expect("test: renewal should succeed");

        assert_eq!(token.original_serial, "cert-priv-1");
        assert_eq!(token.scope, GraceScope::Private);

        // Validate the token
        let validated = mgr
            .validate_token(&token.token_id)
            .await
            .expect("test: token should be valid");
        assert_eq!(validated.token_id, token.token_id);
    }

    #[tokio::test]
    async fn test_anonymous_scope_no_grace() {
        let mgr = GracePeriodManager::new();

        assert!(!mgr.is_within_grace_period(recent_expiry(), &GraceScope::Anonymous));

        let err = mgr
            .request_renewal("cert-anon-1", recent_expiry(), &GraceScope::Anonymous)
            .await;
        assert!(err.is_err(), "Anonymous scope should have no grace period");
    }

    #[tokio::test]
    async fn test_expired_past_grace_window() {
        let mgr = GracePeriodManager::new();

        // Certificate expired 60 days ago -- past 30-day public grace
        assert!(!mgr.is_within_grace_period(old_expiry(), &GraceScope::Public));

        let err = mgr
            .request_renewal("cert-old", old_expiry(), &GraceScope::Public)
            .await;
        assert!(err.is_err(), "Past-grace cert should be rejected");
    }

    #[tokio::test]
    async fn test_max_renewals_enforced() {
        let config = GracePeriodConfig {
            max_renewals_per_cert: 2,
            ..Default::default()
        };
        let mgr = GracePeriodManager::with_config(config);

        let expiry = recent_expiry();

        // First two renewals succeed
        mgr.request_renewal("cert-limit", expiry, &GraceScope::Private)
            .await
            .expect("test: renewal 1");
        mgr.request_renewal("cert-limit", expiry, &GraceScope::Private)
            .await
            .expect("test: renewal 2");

        // Third exceeds limit
        let err = mgr
            .request_renewal("cert-limit", expiry, &GraceScope::Private)
            .await;
        assert!(err.is_err(), "Third renewal should be rejected");

        assert_eq!(mgr.get_renewal_count("cert-limit").await, 2);
    }

    #[tokio::test]
    async fn test_consume_token() {
        let mgr = GracePeriodManager::new();

        let token = mgr
            .request_renewal("cert-consume", recent_expiry(), &GraceScope::Public)
            .await
            .expect("test: renewal");

        let consumed = mgr
            .consume_token(&token.token_id)
            .await
            .expect("test: consume");
        assert_eq!(consumed.original_serial, "cert-consume");

        // Second consume should fail (token already used)
        let err = mgr.consume_token(&token.token_id).await;
        assert!(err.is_err(), "Double consumption should fail");
    }
}
