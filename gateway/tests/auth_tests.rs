// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::sync::Arc;
use std::time::Duration;

use gateway::auth::{AuthManager, AuthResult};
use gateway::bootstrap::BootstrapHandler;
use hypermesh_lib::PrivacyMode;

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
    mgr.register_session("tok-123".into(), "alice".into(), PrivacyMode::PUBLIC);

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
    let body: gateway::bootstrap::BootstrapResponse =
        serde_json::from_slice(resp.body()).expect("test: valid json");
    let token = body.bootstrap_token.clone();

    let mgr = AuthManager::new(Some(handler));
    let mut headers = http::HeaderMap::new();
    let val = format!("Bearer {token}");
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
    mgr.register_session("s-1".into(), "bob".into(), PrivacyMode::PRIVATE);

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
    mgr.register_session("old-tok".into(), "eve".into(), PrivacyMode::PUBLIC);

    // Sleep briefly so the session ages past 0ms
    std::thread::sleep(Duration::from_millis(10));
    let removed = mgr.cleanup_sessions(Duration::from_millis(1));
    assert_eq!(removed, 1, "expired session should be cleaned up");
    assert_eq!(mgr.auth_stats().active_sessions, 0);
}

#[test]
fn cleanup_retains_fresh_sessions() {
    let mgr = AuthManager::new(None);
    mgr.register_session("fresh-tok".into(), "carol".into(), PrivacyMode::PUBLIC);

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
