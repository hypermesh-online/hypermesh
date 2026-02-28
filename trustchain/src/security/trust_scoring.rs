// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Binary Authentication Module
//!
//! PoS (Proof of State) is binary authentication: "Something is either authentic
//! or it isn't. There is no voting, no quorum, and no leader election."
//!
//! This module provides binary pass/fail authentication based on Byzantine
//! detection data. No trust scores, no reputation floats, no time-decay.

use std::collections::HashMap;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::byzantine::{ByzantineDetector, ByzantineViolation};
use crate::errors::Result as TrustChainResult;

/// Binary authentication result -- pass or fail
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticationResult {
    /// Whether the entity passed authentication
    pub authenticated: bool,
    /// Reason for failure (None if authenticated)
    pub reason: Option<String>,
    /// When this result was computed
    pub checked_at: SystemTime,
}

/// Binary authenticator that checks revocation status from Byzantine detection
pub struct BinaryAuthenticator {
    /// Nodes that have been revoked (node_id -> reason)
    revoked: RwLock<HashMap<String, String>>,
}

impl BinaryAuthenticator {
    /// Create a new binary authenticator
    pub fn new() -> Self {
        Self {
            revoked: RwLock::new(HashMap::new()),
        }
    }

    /// Authenticate a node -- binary pass/fail
    pub async fn authenticate(&self, node_id: &str) -> TrustChainResult<AuthenticationResult> {
        let revoked = self.revoked.read().await;
        if let Some(reason) = revoked.get(node_id) {
            warn!("Node {} REJECTED: {}", node_id, reason);
            Ok(AuthenticationResult {
                authenticated: false,
                reason: Some(reason.clone()),
                checked_at: SystemTime::now(),
            })
        } else {
            Ok(AuthenticationResult {
                authenticated: true,
                reason: None,
                checked_at: SystemTime::now(),
            })
        }
    }

    /// Revoke a node based on Byzantine violation
    pub async fn revoke(
        &self,
        node_id: &str,
        violation: &ByzantineViolation,
    ) -> TrustChainResult<AuthenticationResult> {
        let reason = format!("Revoked due to Byzantine violation: {:?}", violation);
        {
            let mut revoked = self.revoked.write().await;
            revoked.insert(node_id.to_string(), reason.clone());
        }
        warn!("Node {} REVOKED: {}", node_id, reason);
        Ok(AuthenticationResult {
            authenticated: false,
            reason: Some(reason),
            checked_at: SystemTime::now(),
        })
    }

    /// Check if a node is revoked
    pub async fn is_revoked(&self, node_id: &str) -> bool {
        let revoked = self.revoked.read().await;
        revoked.contains_key(node_id)
    }

    /// Remove revocation (re-authenticate after investigation)
    pub async fn unrevoke(&self, node_id: &str) {
        let mut revoked = self.revoked.write().await;
        if revoked.remove(node_id).is_some() {
            info!("Node {} revocation lifted", node_id);
        }
    }

    /// Get count of revoked nodes
    pub async fn revoked_count(&self) -> usize {
        let revoked = self.revoked.read().await;
        revoked.len()
    }

    /// Ingest violations from a ByzantineDetector and revoke if warranted
    pub async fn ingest_from_detector(
        &self,
        detector: &ByzantineDetector,
        node_id: &str,
    ) -> TrustChainResult<AuthenticationResult> {
        let summary = detector.get_detection_summary().await?;
        for suspicious in &summary.top_suspicious_nodes {
            if suspicious.node_id == node_id && suspicious.recent_violations > 0 {
                let reason = format!(
                    "Revoked via detector: {} recent violations",
                    suspicious.recent_violations
                );
                let mut revoked = self.revoked.write().await;
                revoked.insert(node_id.to_string(), reason);
            }
        }
        self.authenticate(node_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_new_node_is_authenticated() {
        let auth = BinaryAuthenticator::new();
        let result = auth.authenticate("new-node").await.expect("test");
        assert!(result.authenticated);
        assert!(result.reason.is_none());
    }

    #[tokio::test]
    async fn test_revoked_node_fails_auth() {
        let auth = BinaryAuthenticator::new();
        let violation = ByzantineViolation::TimeManipulation {
            time_offset: Duration::from_secs(600),
            suspicious_nonce: 42,
        };
        let result = auth.revoke("bad-node", &violation).await.expect("test");
        assert!(!result.authenticated);
        assert!(result.reason.is_some());

        // Subsequent auth should also fail
        let result2 = auth.authenticate("bad-node").await.expect("test");
        assert!(!result2.authenticated);
    }

    #[tokio::test]
    async fn test_unrevoke_restores_auth() {
        let auth = BinaryAuthenticator::new();
        let violation = ByzantineViolation::InvalidStakeSignature {
            stake_holder_id: "bad".to_string(),
        };
        auth.revoke("node-1", &violation).await.expect("test");
        assert!(auth.is_revoked("node-1").await);

        auth.unrevoke("node-1").await;
        assert!(!auth.is_revoked("node-1").await);

        let result = auth.authenticate("node-1").await.expect("test");
        assert!(result.authenticated);
    }

    #[tokio::test]
    async fn test_revoked_count() {
        let auth = BinaryAuthenticator::new();
        assert_eq!(auth.revoked_count().await, 0);

        let v = ByzantineViolation::WorkCheating {
            claimed_power: 999,
            actual_power: 1,
        };
        auth.revoke("node-a", &v).await.expect("test");
        auth.revoke("node-b", &v).await.expect("test");
        assert_eq!(auth.revoked_count().await, 2);
    }

    #[tokio::test]
    async fn test_different_violation_types_all_revoke() {
        let auth = BinaryAuthenticator::new();

        let violations: Vec<ByzantineViolation> = vec![
            ByzantineViolation::InvalidStakeSignature {
                stake_holder_id: "x".to_string(),
            },
            ByzantineViolation::StorageFalsification {
                claimed_storage: 1000,
                actual_storage: 10,
            },
            ByzantineViolation::WorkCheating {
                claimed_power: 999,
                actual_power: 1,
            },
            ByzantineViolation::TimeManipulation {
                time_offset: Duration::from_secs(600),
                suspicious_nonce: 1,
            },
            ByzantineViolation::ReplayAttack {
                original_timestamp: SystemTime::now(),
            },
        ];

        for (i, v) in violations.iter().enumerate() {
            let node = format!("node-{}", i);
            let result = auth.revoke(&node, v).await.expect("test");
            assert!(!result.authenticated, "Violation {:?} should revoke", v);
        }
        assert_eq!(auth.revoked_count().await, 5);
    }
}
