// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HTTP/3 bootstrap handler for new agents.
//!
//! When a new agent (device or node) first contacts the gateway via HTTP/3,
//! it receives a `BootstrapResponse` containing the STOQ endpoint address,
//! a single-use bootstrap token, and supported privacy modes. The agent
//! then uses this information to establish a direct STOQ connection.

use bytes::Bytes;
use chrono::{Duration as ChronoDuration, Utc};
use dashmap::DashMap;
use http::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Bootstrap response returned to clients requesting STOQ connection info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResponse {
    /// STOQ endpoint address the client should connect to (e.g. "[::]:8444").
    pub stoq_endpoint: String,
    /// Single-use token the client must present when connecting via STOQ.
    pub bootstrap_token: String,
    /// Privacy modes supported by this gateway.
    pub supported_privacy_modes: Vec<String>,
    /// ISO-8601 expiration timestamp for the token.
    pub expires_at: String,
}

/// A stored bootstrap token with metadata and expiry.
struct StoredToken {
    _token: String,
    _created_at: chrono::DateTime<Utc>,
    expires_at: chrono::DateTime<Utc>,
    _client_id: Option<String>,
}

/// Manages the HTTP/3 bootstrap flow for STOQ onboarding.
///
/// Issues time-limited, single-use tokens that clients present when
/// upgrading from HTTP/3 to STOQ. Expired tokens are cleaned up
/// lazily during token issuance.
pub struct BootstrapHandler {
    stoq_endpoint: String,
    tokens: Arc<DashMap<String, StoredToken>>,
    token_ttl: ChronoDuration,
}

impl BootstrapHandler {
    /// Create a new bootstrap handler.
    ///
    /// * `stoq_endpoint` - The STOQ address clients should connect to.
    /// * `token_ttl_secs` - Lifetime of each bootstrap token in seconds.
    pub fn new(stoq_endpoint: String, token_ttl_secs: i64) -> Self {
        Self {
            stoq_endpoint,
            tokens: Arc::new(DashMap::new()),
            token_ttl: ChronoDuration::seconds(token_ttl_secs),
        }
    }

    /// Handle a bootstrap request.
    ///
    /// Generates a new single-use token, stores it, and returns an HTTP
    /// response containing the STOQ endpoint, token, and supported privacy
    /// modes.
    ///
    /// Optionally associates a `client_id` with the token for audit purposes.
    pub fn handle_bootstrap(&self, client_id: Option<&str>) -> Response<Bytes> {
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + self.token_ttl;

        self.tokens.insert(
            token.clone(),
            StoredToken {
                _token: token.clone(),
                _created_at: now,
                expires_at,
                _client_id: client_id.map(String::from),
            },
        );

        // Periodically clean up expired tokens on issuance path
        self.cleanup_expired();

        let resp = BootstrapResponse {
            stoq_endpoint: self.stoq_endpoint.clone(),
            bootstrap_token: token,
            supported_privacy_modes: vec!["anonymous".into(), "private".into(), "public".into()],
            expires_at: expires_at.to_rfc3339(),
        };

        info!(
            endpoint = %self.stoq_endpoint,
            active_tokens = self.tokens.len(),
            "Bootstrap response issued"
        );

        let body =
            serde_json::to_vec(&resp).expect("BootstrapResponse serialization should not fail");

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Bytes::from(body))
            .expect("building bootstrap response should not fail")
    }

    /// Validate a previously issued bootstrap token.
    ///
    /// Tokens are single-use: a successful validation removes the token
    /// from the store. Returns `false` if the token does not exist or
    /// has expired.
    pub fn validate_token(&self, token: &str) -> bool {
        if let Some((_, stored)) = self.tokens.remove(token) {
            let valid = stored.expires_at > Utc::now();
            if !valid {
                debug!(token = %token, "Bootstrap token expired");
            }
            valid
        } else {
            debug!(token = %token, "Bootstrap token not found");
            false
        }
    }

    /// Remove all expired tokens from the store.
    pub fn cleanup_expired(&self) {
        let now = Utc::now();
        let before = self.tokens.len();
        self.tokens.retain(|_, v| v.expires_at > now);
        let removed = before - self.tokens.len();
        if removed > 0 {
            debug!("Cleaned up {} expired bootstrap tokens", removed);
        }
    }

    /// Number of active (non-expired) tokens currently stored.
    pub fn active_token_count(&self) -> usize {
        self.tokens.len()
    }

    /// Get the configured STOQ endpoint address.
    pub fn stoq_endpoint(&self) -> &str {
        &self.stoq_endpoint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_creation() {
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 300);
        assert_eq!(handler.stoq_endpoint(), "[::]:8444");
        assert_eq!(handler.active_token_count(), 0);
    }

    #[test]
    fn bootstrap_response_generation() {
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 300);
        let resp = handler.handle_bootstrap(None);

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").map(|v| v.to_str().ok()),
            Some(Some("application/json"))
        );

        let body: BootstrapResponse =
            serde_json::from_slice(resp.body()).expect("test: valid json body");
        assert_eq!(body.stoq_endpoint, "[::]:8444");
        assert!(!body.bootstrap_token.is_empty());
        assert_eq!(body.supported_privacy_modes.len(), 3);
        assert!(body
            .supported_privacy_modes
            .contains(&"anonymous".to_string()));
        assert!(body
            .supported_privacy_modes
            .contains(&"private".to_string()));
        assert!(body.supported_privacy_modes.contains(&"public".to_string()));
        assert!(!body.expires_at.is_empty());
    }

    #[test]
    fn bootstrap_increments_token_count() {
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 300);
        assert_eq!(handler.active_token_count(), 0);

        handler.handle_bootstrap(None);
        assert_eq!(handler.active_token_count(), 1);

        handler.handle_bootstrap(Some("client-A"));
        assert_eq!(handler.active_token_count(), 2);
    }

    #[test]
    fn token_validation_succeeds_for_valid_token() {
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 300);
        let resp = handler.handle_bootstrap(None);
        let body: BootstrapResponse =
            serde_json::from_slice(resp.body()).expect("test: valid json");

        assert!(handler.validate_token(&body.bootstrap_token));
        // Token count decreases after consumption
        assert_eq!(handler.active_token_count(), 0);
    }

    #[test]
    fn token_validation_fails_for_unknown_token() {
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 300);
        assert!(!handler.validate_token("nonexistent-token"));
    }

    #[test]
    fn token_is_single_use() {
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 300);
        let resp = handler.handle_bootstrap(None);
        let body: BootstrapResponse =
            serde_json::from_slice(resp.body()).expect("test: valid json");

        // First use succeeds
        assert!(handler.validate_token(&body.bootstrap_token));
        // Second use fails (consumed)
        assert!(!handler.validate_token(&body.bootstrap_token));
    }

    #[test]
    fn expired_token_validation_fails() {
        // Create handler with 0s TTL so tokens expire immediately
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 0);
        let resp = handler.handle_bootstrap(None);
        let body: BootstrapResponse =
            serde_json::from_slice(resp.body()).expect("test: valid json");

        // Sleep briefly to ensure the token has expired
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(!handler.validate_token(&body.bootstrap_token));
    }

    #[test]
    fn cleanup_removes_expired_tokens() {
        // 0s TTL means tokens expire immediately
        let handler = BootstrapHandler::new("[::]:8444".to_string(), 0);
        handler.handle_bootstrap(None);
        handler.handle_bootstrap(None);

        std::thread::sleep(std::time::Duration::from_millis(10));

        // Tokens should still be in the map (cleanup is lazy)
        // After explicit cleanup they should be gone
        handler.cleanup_expired();
        assert_eq!(handler.active_token_count(), 0);
    }

    #[test]
    fn bootstrap_response_serde_roundtrip() {
        let resp = BootstrapResponse {
            stoq_endpoint: "[::]:8444".to_string(),
            bootstrap_token: "test-token-123".to_string(),
            supported_privacy_modes: vec!["anonymous".into(), "public".into()],
            expires_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).expect("test: serialize");
        let parsed: BootstrapResponse = serde_json::from_str(&json).expect("test: deserialize");

        assert_eq!(parsed.stoq_endpoint, resp.stoq_endpoint);
        assert_eq!(parsed.bootstrap_token, resp.bootstrap_token);
        assert_eq!(parsed.supported_privacy_modes, resp.supported_privacy_modes);
        assert_eq!(parsed.expires_at, resp.expires_at);
    }
}
