// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase K.2 — Remote dashboard proxy + 3 modes integration tests.
//!
//! K.2 ships the proxy structure and the capability-token-forwarding
//! contract. Wire encryption + authentication are STOQ's responsibility
//! (X25519MLKEM768 QUIC key exchange + FALCON-PoS handshake) — the
//! gateway is a byte-forwarder over an already-secure stream. These
//! tests exercise the proxy plumbing and capability-token handling
//! without a real STOQ peer, via [`MockStreamForwarder`].

use std::sync::Arc;

use gateway::remote_proxy::{
    DashboardServeMode, ForwardedRequest, MockStreamForwarder, RemoteDashboardProxy,
    RemoteProxyError, RemoteProxyManager, StoqStreamForwarder,
};

// ---------- 1. capability-token forwarding (proxy structure) ----------

#[tokio::test]
async fn test_capability_token_forwarded_to_target() {
    // The proxy is constructed with an opaque capability token. It
    // embeds the token in every forwarded envelope so the target node
    // can validate it. This test confirms the byte-forwarding contract.
    let token = b"mock-capability-token-bytes".to_vec();
    let mock = Arc::new(MockStreamForwarder::new());
    let forwarder: Arc<dyn StoqStreamForwarder> = mock.clone();
    let proxy = RemoteDashboardProxy::new(
        "yourname.hypermesh".into(),
        token.clone(),
        DashboardServeMode::PrivateDomain,
        forwarder,
    )
    .expect("proxy");
    assert_eq!(proxy.token_byte_len(), token.len());
    let req = ForwardedRequest {
        method: "GET".into(),
        path: "/dashboard".into(),
        body: vec![],
    };
    let resp = proxy.forward_request(req).await.expect("forward");
    assert_eq!(resp.status, 200);

    // The capability token must reach the target verbatim (hex-encoded
    // inside the envelope) so the target can authenticate.
    let (target, bytes) = mock.last_forwarded().await.expect("recorded");
    assert_eq!(target, "yourname.hypermesh");
    let envelope_str = String::from_utf8_lossy(&bytes);
    assert!(
        envelope_str.contains(&hex::encode(&token)),
        "capability token must appear in forwarded envelope",
    );
}

#[tokio::test]
async fn test_capability_token_revoked_session_rejected() {
    // Closing the proxy mimics a revoked-session rejection — subsequent
    // forwards fail closed.
    let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
    let proxy = RemoteDashboardProxy::new(
        "node.example".into(),
        b"tok".to_vec(),
        DashboardServeMode::TrustProxy,
        forwarder,
    )
    .expect("proxy");
    proxy.close().await;
    let req = ForwardedRequest {
        method: "GET".into(),
        path: "/".into(),
        body: vec![],
    };
    let res = proxy.forward_request(req).await;
    assert!(matches!(res, Err(RemoteProxyError::SessionClosed)));
}

// ---------- 2. proxy forwards request bytes via STOQ forwarder ----------

#[tokio::test]
async fn test_remote_proxy_forwards_request_via_stoq() {
    // The architectural commitment of K.2 is that the gateway forwards
    // request bytes to the configured STOQ forwarder, addressed at the
    // configured target. Wire encryption is STOQ's job — the gateway
    // does not encrypt anything itself.
    let mock = Arc::new(MockStreamForwarder::new());
    let forwarder: Arc<dyn StoqStreamForwarder> = mock.clone();
    let proxy = RemoteDashboardProxy::new(
        "trust.hypermesh.online".into(),
        b"session-BBB".to_vec(),
        DashboardServeMode::TrustProxy,
        forwarder,
    )
    .expect("proxy");
    assert_eq!(proxy.session_id().len(), 16); // 8 bytes hex

    let req = ForwardedRequest {
        method: "POST".into(),
        path: "/api/echo".into(),
        body: b"sensitive-payload".to_vec(),
    };
    let resp = proxy.forward_request(req).await.expect("forward");
    assert_eq!(resp.status, 200);
    assert!(String::from_utf8_lossy(&resp.body).contains("trust.hypermesh.online"));

    // The forwarder saw the bytes addressed to the right target.
    let (target, bytes) = mock.last_forwarded().await.expect("recorded");
    assert_eq!(target, "trust.hypermesh.online");
    assert!(!bytes.is_empty(), "request bytes must be non-empty");
}

// ---------- 3. invalid token rejected ----------

#[tokio::test]
async fn test_remote_proxy_rejects_invalid_token() {
    // Empty token bytes are rejected at construction time — the gateway
    // will not open a session for an unauthenticated caller.
    let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
    let res = RemoteDashboardProxy::new(
        "node.example".into(),
        vec![],
        DashboardServeMode::TrustProxy,
        forwarder,
    );
    let err = match res {
        Ok(_) => panic!("expected error for empty token"),
        Err(e) => e,
    };
    assert!(matches!(err, RemoteProxyError::InvalidToken(_)));
}

// ---------- 4. all 3 modes parse + round-trip ----------

#[test]
fn test_three_modes_configured() {
    // Mode 1: trust-proxy
    assert_eq!(
        DashboardServeMode::parse("trust-proxy"),
        Some(DashboardServeMode::TrustProxy),
    );
    // Mode 2: private-domain
    assert_eq!(
        DashboardServeMode::parse("private-domain"),
        Some(DashboardServeMode::PrivateDomain),
    );
    // Mode 3: self-hosted
    assert_eq!(
        DashboardServeMode::parse("self-hosted"),
        Some(DashboardServeMode::SelfHosted),
    );
    // Aliases
    assert_eq!(
        DashboardServeMode::parse("trust"),
        Some(DashboardServeMode::TrustProxy),
    );
    assert_eq!(
        DashboardServeMode::parse("private"),
        Some(DashboardServeMode::PrivateDomain),
    );
    // Unknown
    assert_eq!(DashboardServeMode::parse("garbage"), None);
}

#[test]
fn test_three_modes_have_stable_strings() {
    for mode in [
        DashboardServeMode::TrustProxy,
        DashboardServeMode::PrivateDomain,
        DashboardServeMode::SelfHosted,
    ] {
        let s = mode.as_str();
        assert!(!s.is_empty());
        // Round-trip.
        assert_eq!(DashboardServeMode::parse(s), Some(mode));
    }
}

// ---------- 5. proxy manager session lifecycle ----------

#[tokio::test]
async fn test_proxy_manager_lifecycle() {
    let mgr = RemoteProxyManager::new();
    assert!(mgr.is_empty().await);
    let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());

    // Start three sessions, each with a different mode.
    let p1 = Arc::new(
        RemoteDashboardProxy::new(
            "trust.hypermesh.online".into(),
            b"tA".to_vec(),
            DashboardServeMode::TrustProxy,
            forwarder.clone(),
        )
        .expect("p1"),
    );
    let p2 = Arc::new(
        RemoteDashboardProxy::new(
            "yourname.hypermesh".into(),
            b"tB".to_vec(),
            DashboardServeMode::PrivateDomain,
            forwarder.clone(),
        )
        .expect("p2"),
    );
    let p3 = Arc::new(
        RemoteDashboardProxy::new(
            "selfhost.example".into(),
            b"tC".to_vec(),
            DashboardServeMode::SelfHosted,
            forwarder,
        )
        .expect("p3"),
    );

    let s1 = mgr.register(p1.clone()).await;
    let s2 = mgr.register(p2.clone()).await;
    let s3 = mgr.register(p3.clone()).await;
    assert_eq!(mgr.len().await, 3);
    assert_ne!(s1, s2);
    assert_ne!(s2, s3);

    // Lookup.
    let f = mgr.get(&s2).await.expect("s2 present");
    assert_eq!(f.target_node(), "yourname.hypermesh");

    // Close.
    assert!(mgr.close(&s1).await);
    assert_eq!(mgr.len().await, 2);

    // Reap: close p2 underneath the manager and reap.
    p2.close().await;
    let reaped = mgr.reap_inactive().await;
    assert_eq!(reaped, 1);
    assert_eq!(mgr.len().await, 1);
}
