// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase K.2 — Remote dashboard proxy + 3 modes integration tests.

use std::sync::Arc;

use gateway::remote_proxy::{
    DashboardServeMode, ForwardedRequest, KyberAesTunnel, KyberKemTunnel, MockTunnel,
    RemoteDashboardProxy, RemoteProxyError, RemoteProxyManager,
};

// ---------- 1. capability-token forwarding (proxy structure) ----------

#[tokio::test]
async fn test_capability_token_validation_in_ipc_middleware() {
    // The proxy is constructed with an opaque session token. The
    // gateway forwards the token bytes to the target (which validates
    // them) — this test exercises the byte-forwarding contract.
    let token = b"mock-capability-token-bytes".to_vec();
    let proxy = RemoteDashboardProxy::new(
        "yourname.hypermesh".into(),
        token.clone(),
        DashboardServeMode::PrivateDomain,
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
}

#[tokio::test]
async fn test_capability_token_revoked_session_rejected() {
    // Closing the proxy mimics a revoked-session rejection at the
    // gateway layer — subsequent forwards fail closed.
    let proxy = RemoteDashboardProxy::new(
        "node.example".into(),
        b"tok".to_vec(),
        DashboardServeMode::TrustProxy,
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

#[tokio::test]
async fn test_capability_token_expired_rejected() {
    // Expiry surfaces through the tunnel as SessionClosed once the
    // tunnel is torn down. (The token-expiry check itself happens at
    // the daemon when forward_request reaches the target — gateway is
    // a byte-forwarder.)
    let tunnel: Arc<dyn KyberAesTunnel> =
        Arc::new(MockTunnel::new("expired".into(), b"tok"));
    tunnel.close().await;
    let proxy = RemoteDashboardProxy::with_tunnel(
        "node.example".into(),
        b"tok".to_vec(),
        DashboardServeMode::SelfHosted,
        tunnel,
    )
    .expect("construct");
    let req = ForwardedRequest {
        method: "GET".into(),
        path: "/".into(),
        body: vec![],
    };
    let res = proxy.forward_request(req).await;
    assert!(matches!(res, Err(RemoteProxyError::SessionClosed)));
}

// ---------- 2. Kyber→AES tunnel forwards request as ciphertext ----------

#[tokio::test]
async fn test_remote_proxy_forwards_request_with_kyber_tunnel() {
    // K.2 ships MockTunnel for the proxy structure; the architectural
    // commitment is that *gateway never sees plaintext*. We confirm
    // this by capturing the seal_request output and asserting it
    // differs from the plaintext.
    let tunnel = Arc::new(MockTunnel::new("captured".into(), b"session-AAA"));
    let plaintext = b"hello dashboard";
    let ciphertext = tunnel.seal_request(plaintext).await.expect("seal");
    assert_ne!(ciphertext, plaintext);
    // Encryption is reversible from the same key.
    let plain_back = tunnel.open_response(&ciphertext).await.expect("open");
    assert_eq!(plain_back, plaintext);

    // Through the proxy, request bodies likewise pass through the
    // tunnel. We can't observe the wire ciphertext directly because
    // it's hidden inside forward_request, but we can verify the
    // contract via tunnel_id and round-tripping the response.
    let proxy = RemoteDashboardProxy::with_tunnel(
        "trust.hypermesh.online".into(),
        b"session-BBB".to_vec(),
        DashboardServeMode::TrustProxy,
        tunnel,
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
}

// ---------- 3. invalid token rejected ----------

#[tokio::test]
async fn test_remote_proxy_rejects_invalid_token() {
    // Empty token bytes are rejected at construction time — the
    // gateway will not open a tunnel for an unauthenticated session.
    let res = RemoteDashboardProxy::new(
        "node.example".into(),
        vec![],
        DashboardServeMode::TrustProxy,
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

    // Start three sessions, each with a different mode.
    let p1 = Arc::new(
        RemoteDashboardProxy::new(
            "trust.hypermesh.online".into(),
            b"tA".to_vec(),
            DashboardServeMode::TrustProxy,
        )
        .expect("p1"),
    );
    let p2 = Arc::new(
        RemoteDashboardProxy::new(
            "yourname.hypermesh".into(),
            b"tB".to_vec(),
            DashboardServeMode::PrivateDomain,
        )
        .expect("p2"),
    );
    let p3 = Arc::new(
        RemoteDashboardProxy::new(
            "selfhost.example".into(),
            b"tC".to_vec(),
            DashboardServeMode::SelfHosted,
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

// ---------- 6. KyberKemTunnel (production stub) ----------

#[test]
fn test_kyber_kem_tunnel_is_stub_pending_k2_5() {
    // The production tunnel is intentionally stubbed — K.2.5 wires it
    // against trustchain::crypto::KyberCrypto. Until then, calling
    // KyberKemTunnel::new returns an error directing callers to use
    // MockTunnel for tests.
    let res = KyberKemTunnel::new("target.example", b"some-token");
    assert!(res.is_err());
    let err = res.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("K.2.5"), "error msg should reference K.2.5: {msg}");
}
