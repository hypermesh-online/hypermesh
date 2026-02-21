// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Authentication manager for gateway connections.
//!
//! Provides authentication for both HTTP/3 requests (via Bearer tokens)
//! and STOQ connections (via privacy-mode-aware identity). Supports
//! session management with time-based expiry and bootstrap token
//! validation through the `BootstrapHandler`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{debug, warn};

use hypermesh_lib::PrivacyMode;

use crate::bootstrap::BootstrapHandler;

/// Result of an authentication attempt.
#[derive(Debug, Clone)]
pub enum AuthResult {
    /// Client authenticated with a verified identity.
    Authenticated {
        /// Opaque identity string (e.g. session owner, STOQ connection ID).
        identity: String,
        /// Privacy mode associated with the session.
        privacy_mode: PrivacyMode,
    },
    /// Client must complete the bootstrap flow at `/bootstrap` first.
    BootstrapRequired,
    /// Authentication was explicitly rejected.
    Rejected {
        /// Human-readable rejection reason.
        reason: String,
    },
    /// No authentication required (anonymous access).
    Anonymous,
}

impl AuthResult {
    /// Returns `true` when the caller is permitted to proceed.
    pub fn is_authenticated(&self) -> bool {
        matches!(self, Self::Authenticated { .. } | Self::Anonymous)
    }
}

/// A session registered via `AuthManager::register_session`.
struct AuthenticatedSession {
    identity: String,
    privacy_mode: PrivacyMode,
    created_at: Instant,
}

/// Atomic counters for authentication statistics.
struct AuthStats {
    total_attempts: AtomicU64,
    successful: AtomicU64,
    rejected: AtomicU64,
    anonymous: AtomicU64,
}

impl AuthStats {
    fn new() -> Self {
        Self {
            total_attempts: AtomicU64::new(0),
            successful: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            anonymous: AtomicU64::new(0),
        }
    }
}

/// Point-in-time snapshot of authentication statistics.
#[derive(Debug, Clone)]
pub struct AuthStatsSnapshot {
    pub total_attempts: u64,
    pub successful: u64,
    pub rejected: u64,
    pub anonymous: u64,
    pub active_sessions: usize,
}

/// Manages authentication for gateway connections.
///
/// Supports three authentication paths:
/// 1. **Bearer token** -- looked up in the session store or validated via bootstrap handler.
/// 2. **STOQ identity** -- derived from the connection's privacy mode.
/// 3. **Anonymous** -- no credentials required (exempt paths, anonymous privacy mode).
pub struct AuthManager {
    bootstrap_handler: Option<Arc<BootstrapHandler>>,
    sessions: Arc<DashMap<String, AuthenticatedSession>>,
    stats: Arc<AuthStats>,
}

impl AuthManager {
    /// Create a new authentication manager.
    ///
    /// If `bootstrap_handler` is `Some`, bootstrap tokens are validated
    /// against it when they do not match an existing session.
    pub fn new(bootstrap_handler: Option<Arc<BootstrapHandler>>) -> Self {
        Self {
            bootstrap_handler,
            sessions: Arc::new(DashMap::new()),
            stats: Arc::new(AuthStats::new()),
        }
    }

    /// Authenticate an HTTP/3 request by inspecting its headers.
    ///
    /// Resolution order:
    /// 1. `Authorization: Bearer <token>` -- session lookup, then bootstrap validation.
    /// 2. No header -- anonymous access.
    pub fn authenticate_http3(&self, headers: &http::HeaderMap) -> AuthResult {
        self.stats.total_attempts.fetch_add(1, Ordering::Relaxed);

        // Check for Authorization header (Bearer token)
        if let Some(auth_header) = headers.get("authorization") {
            let auth_str = match auth_header.to_str() {
                Ok(s) => s,
                Err(_) => {
                    self.stats.rejected.fetch_add(1, Ordering::Relaxed);
                    return AuthResult::Rejected {
                        reason: "invalid authorization header encoding".into(),
                    };
                }
            };

            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return self.validate_bearer_token(token);
            }

            // Header present but not Bearer scheme
            self.stats.rejected.fetch_add(1, Ordering::Relaxed);
            return AuthResult::Rejected {
                reason: "unsupported authorization scheme (expected Bearer)".into(),
            };
        }

        // No auth header -- anonymous access
        self.stats.anonymous.fetch_add(1, Ordering::Relaxed);
        AuthResult::Anonymous
    }

    /// Authenticate a STOQ connection based on its privacy mode.
    ///
    /// * `Anonymous` privacy mode skips authentication entirely.
    /// * `Private` / `Public` modes derive identity from the connection ID.
    pub fn authenticate_stoq(
        &self,
        privacy_mode: PrivacyMode,
        connection_id: &str,
    ) -> AuthResult {
        self.stats.total_attempts.fetch_add(1, Ordering::Relaxed);

        if privacy_mode == PrivacyMode::ANONYMOUS {
            self.stats.anonymous.fetch_add(1, Ordering::Relaxed);
            return AuthResult::Anonymous;
        }

        // For Private/Public the STOQ handshake itself serves as identity
        self.stats.successful.fetch_add(1, Ordering::Relaxed);
        AuthResult::Authenticated {
            identity: format!("stoq:{}", connection_id),
            privacy_mode,
        }
    }

    /// Register an authenticated session that can be looked up by token.
    pub fn register_session(
        &self,
        token: String,
        identity: String,
        privacy_mode: PrivacyMode,
    ) {
        debug!(identity = %identity, "Registering auth session");
        self.sessions.insert(
            token,
            AuthenticatedSession {
                identity,
                privacy_mode,
                created_at: Instant::now(),
            },
        );
    }

    /// Remove sessions older than `max_age`.
    ///
    /// Returns the number of sessions removed.
    pub fn cleanup_sessions(&self, max_age: Duration) -> usize {
        let now = Instant::now();
        let before = self.sessions.len();
        self.sessions
            .retain(|_, s| now.duration_since(s.created_at) < max_age);
        let removed = before - self.sessions.len();
        if removed > 0 {
            debug!("Cleaned up {} expired auth sessions", removed);
        }
        removed
    }

    /// Get a point-in-time snapshot of authentication statistics.
    pub fn auth_stats(&self) -> AuthStatsSnapshot {
        AuthStatsSnapshot {
            total_attempts: self.stats.total_attempts.load(Ordering::Relaxed),
            successful: self.stats.successful.load(Ordering::Relaxed),
            rejected: self.stats.rejected.load(Ordering::Relaxed),
            anonymous: self.stats.anonymous.load(Ordering::Relaxed),
            active_sessions: self.sessions.len(),
        }
    }

    /// Returns `true` for paths that bypass authentication.
    pub fn is_exempt_path(path: &str) -> bool {
        matches!(
            path,
            "/health"
                | "/bootstrap"
                | "/.well-known/acme-challenge"
                | "/favicon.ico"
        )
    }

    // ---- internal helpers ----

    /// Validate a bearer token against sessions and bootstrap handler.
    fn validate_bearer_token(&self, token: &str) -> AuthResult {
        // 1. Check session store
        if let Some(session) = self.sessions.get(token) {
            self.stats.successful.fetch_add(1, Ordering::Relaxed);
            return AuthResult::Authenticated {
                identity: session.identity.clone(),
                privacy_mode: session.privacy_mode,
            };
        }

        // 2. Check bootstrap handler
        if let Some(ref handler) = self.bootstrap_handler {
            if handler.validate_token(token) {
                self.stats.successful.fetch_add(1, Ordering::Relaxed);
                let truncated = &token[..8.min(token.len())];
                return AuthResult::Authenticated {
                    identity: format!("bootstrap:{}", truncated),
                    privacy_mode: PrivacyMode::PUBLIC,
                };
            }
        }

        // 3. Reject
        self.stats.rejected.fetch_add(1, Ordering::Relaxed);
        warn!("Bearer token rejected (not found in sessions or bootstrap)");
        AuthResult::Rejected {
            reason: "invalid bearer token".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AuthResult ---

    #[test]
    fn auth_result_authenticated_is_ok() {
        let r = AuthResult::Authenticated {
            identity: "alice".into(),
            privacy_mode: PrivacyMode::PUBLIC,
        };
        assert!(r.is_authenticated());
    }

    #[test]
    fn auth_result_anonymous_is_ok() {
        assert!(AuthResult::Anonymous.is_authenticated());
    }

    #[test]
    fn auth_result_rejected_is_not_ok() {
        let r = AuthResult::Rejected {
            reason: "bad".into(),
        };
        assert!(!r.is_authenticated());
    }

    #[test]
    fn auth_result_bootstrap_required_is_not_ok() {
        assert!(!AuthResult::BootstrapRequired.is_authenticated());
    }

    // --- is_exempt_path ---

    #[test]
    fn exempt_paths() {
        assert!(AuthManager::is_exempt_path("/health"));
        assert!(AuthManager::is_exempt_path("/bootstrap"));
        assert!(AuthManager::is_exempt_path("/.well-known/acme-challenge"));
        assert!(AuthManager::is_exempt_path("/favicon.ico"));
    }

    #[test]
    fn non_exempt_paths() {
        assert!(!AuthManager::is_exempt_path("/api/v1/trustchain"));
        assert!(!AuthManager::is_exempt_path("/"));
        assert!(!AuthManager::is_exempt_path("/healthz"));
    }

    // --- authenticate_http3 ---

    #[test]
    fn http3_no_header_returns_anonymous() {
        let mgr = AuthManager::new(None);
        let headers = http::HeaderMap::new();
        let result = mgr.authenticate_http3(&headers);
        assert!(matches!(result, AuthResult::Anonymous));
    }

    #[test]
    fn http3_invalid_bearer_token_rejected() {
        let mgr = AuthManager::new(None);
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "authorization",
            http::HeaderValue::from_static("Bearer bad-token"),
        );
        let result = mgr.authenticate_http3(&headers);
        assert!(matches!(result, AuthResult::Rejected { .. }));
    }

    #[test]
    fn http3_session_token_authenticates() {
        let mgr = AuthManager::new(None);
        mgr.register_session(
            "tok-123".into(),
            "alice".into(),
            PrivacyMode::PUBLIC,
        );

        let mut headers = http::HeaderMap::new();
        headers.insert(
            "authorization",
            http::HeaderValue::from_static("Bearer tok-123"),
        );
        let result = mgr.authenticate_http3(&headers);
        match result {
            AuthResult::Authenticated {
                identity,
                privacy_mode,
            } => {
                assert_eq!(identity, "alice");
                assert_eq!(privacy_mode, PrivacyMode::PUBLIC);
            }
            other => unreachable!("expected Authenticated, got {:?}", other),
        }
    }

    #[test]
    fn http3_bootstrap_token_authenticates() {
        let handler = Arc::new(BootstrapHandler::new("[::]:8444".into(), 300));
        let resp = handler.handle_bootstrap(None);
        let body: crate::bootstrap::BootstrapResponse =
            serde_json::from_slice(resp.body()).expect("test: valid json");
        let token = body.bootstrap_token.clone();

        let mgr = AuthManager::new(Some(handler));
        let mut headers = http::HeaderMap::new();
        let val = format!("Bearer {}", token);
        headers.insert(
            "authorization",
            http::HeaderValue::from_str(&val).expect("test: valid header"),
        );
        let result = mgr.authenticate_http3(&headers);
        assert!(
            result.is_authenticated(),
            "bootstrap token should authenticate"
        );
    }

    #[test]
    fn http3_non_bearer_scheme_rejected() {
        let mgr = AuthManager::new(None);
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "authorization",
            http::HeaderValue::from_static("Basic dXNlcjpwYXNz"),
        );
        let result = mgr.authenticate_http3(&headers);
        match result {
            AuthResult::Rejected { reason } => {
                assert!(reason.contains("unsupported authorization scheme"));
            }
            other => unreachable!("expected Rejected, got {:?}", other),
        }
    }

    // --- authenticate_stoq ---

    #[test]
    fn stoq_anonymous_returns_anonymous() {
        let mgr = AuthManager::new(None);
        let result = mgr.authenticate_stoq(PrivacyMode::ANONYMOUS, "conn-1");
        assert!(matches!(result, AuthResult::Anonymous));
    }

    #[test]
    fn stoq_private_returns_authenticated() {
        let mgr = AuthManager::new(None);
        let result = mgr.authenticate_stoq(PrivacyMode::PRIVATE, "conn-42");
        match result {
            AuthResult::Authenticated {
                identity,
                privacy_mode,
            } => {
                assert_eq!(identity, "stoq:conn-42");
                assert_eq!(privacy_mode, PrivacyMode::PRIVATE);
            }
            other => unreachable!("expected Authenticated, got {:?}", other),
        }
    }

    #[test]
    fn stoq_public_returns_authenticated() {
        let mgr = AuthManager::new(None);
        let result = mgr.authenticate_stoq(PrivacyMode::PUBLIC, "conn-99");
        assert!(result.is_authenticated());
    }

    // --- session management ---

    #[test]
    fn register_and_lookup_session() {
        let mgr = AuthManager::new(None);
        mgr.register_session(
            "s-1".into(),
            "bob".into(),
            PrivacyMode::PRIVATE,
        );

        let mut headers = http::HeaderMap::new();
        headers.insert(
            "authorization",
            http::HeaderValue::from_static("Bearer s-1"),
        );
        let result = mgr.authenticate_http3(&headers);
        assert!(result.is_authenticated());
    }

    #[test]
    fn cleanup_removes_expired_sessions() {
        let mgr = AuthManager::new(None);
        mgr.register_session(
            "old-tok".into(),
            "eve".into(),
            PrivacyMode::PUBLIC,
        );

        // Sleep briefly so the session ages past 0ms
        std::thread::sleep(Duration::from_millis(10));
        let removed = mgr.cleanup_sessions(Duration::from_millis(1));
        assert_eq!(removed, 1, "expired session should be cleaned up");
        assert_eq!(mgr.auth_stats().active_sessions, 0);
    }

    #[test]
    fn cleanup_retains_fresh_sessions() {
        let mgr = AuthManager::new(None);
        mgr.register_session(
            "fresh-tok".into(),
            "carol".into(),
            PrivacyMode::PUBLIC,
        );

        let removed = mgr.cleanup_sessions(Duration::from_secs(3600));
        assert_eq!(removed, 0, "fresh session should be retained");
        assert_eq!(mgr.auth_stats().active_sessions, 1);
    }

    // --- stats ---

    #[test]
    fn stats_reflect_operations() {
        let mgr = AuthManager::new(None);

        // Anonymous request
        let headers = http::HeaderMap::new();
        let _ = mgr.authenticate_http3(&headers);

        // Rejected request
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "authorization",
            http::HeaderValue::from_static("Bearer nope"),
        );
        let _ = mgr.authenticate_http3(&headers);

        // STOQ anonymous
        let _ = mgr.authenticate_stoq(PrivacyMode::ANONYMOUS, "c-1");

        // STOQ public
        let _ = mgr.authenticate_stoq(PrivacyMode::PUBLIC, "c-2");

        let snap = mgr.auth_stats();
        assert_eq!(snap.total_attempts, 4);
        assert_eq!(snap.anonymous, 2); // HTTP/3 anonymous + STOQ anonymous
        assert_eq!(snap.rejected, 1);
        assert_eq!(snap.successful, 1); // STOQ public
    }
}
