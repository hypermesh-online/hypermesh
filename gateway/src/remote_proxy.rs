// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase K.2 — remote dashboard proxy.
//!
//! Given a target node URL plus a valid session token, proxies dashboard
//! HTTP/3 traffic from a client browser to the target node over an
//! end-to-end Kyber-1024 KEM → AES-256-GCM tunnel. The gateway acts as a
//! byte-forwarder only; the AES key is derived by the client + target
//! and the gateway never sees plaintext.
//!
//! ## Architecture (3 modes)
//!
//! All three remote-dashboard modes share this proxy:
//!
//! 1. **Trust gateway proxy** — `trust.hypermesh.online` runs the proxy
//!    on the user's behalf after authenticating the user with a FALCON
//!    device-fingerprint signature. Default convenience tier.
//! 2. **Private hypermesh domain** — operator owns the gateway under a
//!    private `yourname.hypermesh` domain (registered via Phase H.1
//!    foundation grant). No third-party relay.
//! 3. **Self-hosted clearnet** — operator runs their own Gateway crate
//!    instance with their own clearnet TLS, capability tokens still
//!    required. Enterprise / governance tier.
//!
//! ## Crypto wiring
//!
//! K.2 ships the proxy structure with a **mock tunnel** for testing.
//! The plumbing for the real Kyber → AES tunnel hooks into
//! `trustchain::crypto::KyberCrypto` (the same primitive used by the
//! asset pipeline) and is staged as K.2.5 — the architectural commitment
//! in this sprint is the proxy + capability-token integration. See
//! [`KyberAesTunnel`] for the trait that K.2.5 will implement against
//! `pqcrypto_kyber::kyber1024`.

#![deny(unsafe_code)]

use crate::error::GatewayError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Error type for remote-proxy operations.
#[derive(Debug, Error)]
pub enum RemoteProxyError {
    /// Token-validation rejection at the target — the session token is
    /// invalid, expired, revoked, or doesn't grant the required scope.
    #[error("session token rejected: {0}")]
    InvalidToken(String),
    /// Tunnel handshake failed — Kyber KEM couldn't establish the
    /// shared secret (e.g. peer rejected our public key).
    #[error("tunnel handshake failed: {0}")]
    HandshakeFailed(String),
    /// Forwarding-time error — wire transport, AES decryption, etc.
    #[error("forward error: {0}")]
    ForwardError(String),
    /// Session expired or was closed.
    #[error("session closed")]
    SessionClosed,
    /// Misconfiguration — caller asked for a target that this gateway
    /// is not configured to reach.
    #[error("target {target} not reachable from this gateway")]
    UnreachableTarget {
        /// The target node URL that couldn't be reached.
        target: String,
    },
}

impl From<RemoteProxyError> for GatewayError {
    fn from(value: RemoteProxyError) -> Self {
        match value {
            RemoteProxyError::InvalidToken(reason) => GatewayError::AuthFailed { reason },
            RemoteProxyError::UnreachableTarget { target } => {
                GatewayError::NoBackendForDomain { domain: target }
            }
            other => GatewayError::ProtocolBridge(other.to_string()),
        }
    }
}

/// Three remote-dashboard modes that share the same proxy implementation.
///
/// Configured via `--dashboard-mode` CLI flag on the gateway binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DashboardServeMode {
    /// Mode 1 — trust.hypermesh.online proxies dashboard for any user
    /// who authenticates with a FALCON device fingerprint signature.
    /// Default convenience tier. The user trusts trust.hypermesh.online
    /// not to intercept (gateway is byte-forwarder only, but operators
    /// of trust.* still see traffic-shape metadata).
    TrustProxy,

    /// Mode 2 — operator self-hosts the gateway under a registered
    /// `yourname.hypermesh` domain. They own the trust path; no
    /// third-party relay. Foundation grant required (Phase H.1).
    PrivateDomain,

    /// Mode 3 — operator runs their own Gateway crate instance with
    /// their own clearnet TLS. Capability tokens still required, but
    /// no HyperMesh-namespace lookup happens. Enterprise / governance
    /// tier.
    SelfHosted,
}

impl DashboardServeMode {
    /// Parse from a CLI flag string. Accepts kebab-case and snake_case.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "trust-proxy" | "trust_proxy" | "trust" => Some(Self::TrustProxy),
            "private-domain" | "private_domain" | "private" => Some(Self::PrivateDomain),
            "self-hosted" | "self_hosted" | "selfhosted" => Some(Self::SelfHosted),
            _ => None,
        }
    }

    /// Stable human-readable label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TrustProxy => "trust-proxy",
            Self::PrivateDomain => "private-domain",
            Self::SelfHosted => "self-hosted",
        }
    }
}

/// Minimal HTTP request shape forwarded over the tunnel.
///
/// Kept deliberately small — full headers/body live in the bytes the
/// caller hands us; this struct is just enough for routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedRequest {
    /// HTTP method, e.g. "GET".
    pub method: String,
    /// Request path including any query string, e.g. "/index.html?v=1".
    pub path: String,
    /// Optional body bytes.
    pub body: Vec<u8>,
}

/// Minimal HTTP response shape returned through the tunnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedResponse {
    /// HTTP status code.
    pub status: u16,
    /// Body bytes.
    pub body: Vec<u8>,
    /// Detected content-type (for browser rendering).
    pub content_type: String,
}

/// Trait abstracting a Kyber-1024 → AES-GCM end-to-end tunnel.
///
/// Implementors negotiate a shared secret during construction and
/// expose authenticated-encryption operations for forwarding traffic.
/// K.2 ships [`MockTunnel`] for the proxy structure + tests. K.2.5
/// will ship `KyberKemTunnel` wired to `trustchain::crypto::KyberCrypto`
/// + `aes_gcm::Aes256Gcm`.
#[async_trait]
pub trait KyberAesTunnel: Send + Sync {
    /// Encrypt a request payload for transit to the target node.
    ///
    /// Returns the ciphertext bytes that should go on the wire. The
    /// gateway never sees the plaintext request body — it only sees
    /// the output of this call.
    async fn seal_request(&self, plaintext: &[u8]) -> Result<Vec<u8>, RemoteProxyError>;

    /// Decrypt a response payload received over the wire.
    async fn open_response(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RemoteProxyError>;

    /// Identifier of this tunnel (for logging / metrics).
    fn tunnel_id(&self) -> &str;

    /// True when the tunnel is still alive. After [`close`] returns
    /// this should be false.
    fn is_active(&self) -> bool;

    /// Tear down the tunnel — explicitly drop the shared key material
    /// and any wire resources.
    async fn close(&self);
}

/// Mock implementation of [`KyberAesTunnel`] for K.2 testing.
///
/// Uses XOR with a deterministic 32-byte "shared secret" derived from
/// the session token. **Not secure** — only used to exercise the
/// proxy plumbing in tests. K.2.5 replaces this with a real Kyber-1024
/// KEM + AES-256-GCM implementation.
pub struct MockTunnel {
    tunnel_id: String,
    shared_secret: [u8; 32],
    active: std::sync::atomic::AtomicBool,
}

impl MockTunnel {
    /// Construct a mock tunnel from the session token bytes.
    pub fn new(tunnel_id: String, session_token: &[u8]) -> Self {
        let shared_secret = *blake3::hash(session_token).as_bytes();
        Self {
            tunnel_id,
            shared_secret,
            active: std::sync::atomic::AtomicBool::new(true),
        }
    }

    fn xor_block(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, b)| b ^ self.shared_secret[i % 32])
            .collect()
    }
}

#[async_trait]
impl KyberAesTunnel for MockTunnel {
    async fn seal_request(&self, plaintext: &[u8]) -> Result<Vec<u8>, RemoteProxyError> {
        if !self.is_active() {
            return Err(RemoteProxyError::SessionClosed);
        }
        Ok(self.xor_block(plaintext))
    }

    async fn open_response(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RemoteProxyError> {
        if !self.is_active() {
            return Err(RemoteProxyError::SessionClosed);
        }
        Ok(self.xor_block(ciphertext))
    }

    fn tunnel_id(&self) -> &str {
        &self.tunnel_id
    }

    fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    async fn close(&self) {
        self.active
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Stub for the future Kyber-1024 + AES-256-GCM tunnel.
///
/// K.2.5 wires this into `trustchain::crypto::KyberCrypto`. The
/// constructor will:
/// 1. Generate a Kyber-1024 keypair for this session.
/// 2. Initiate handshake with the target — exchange FALCON device
///    pubkey + Kyber pub for KEM encapsulation.
/// 3. Derive the AES-256 key from the Kyber shared secret using
///    BLAKE3-HKDF.
/// 4. Hold the AES key in scope for the lifetime of the tunnel; on
///    [`close`] zeroize and drop.
///
/// Until K.2.5 lands, calling [`KyberKemTunnel::new`] returns an error
/// directing operators to use [`MockTunnel`] in test contexts.
#[derive(Debug)]
pub struct KyberKemTunnel {
    _marker: (),
}

impl KyberKemTunnel {
    /// Construct a real Kyber → AES tunnel. Currently returns an
    /// error pending K.2.5 wiring.
    pub fn new(
        _target_node: &str,
        _session_token: &[u8],
    ) -> Result<Self, RemoteProxyError> {
        Err(RemoteProxyError::HandshakeFailed(
            "KyberKemTunnel pending K.2.5 — use MockTunnel until then".into(),
        ))
    }
}

/// Remote dashboard proxy.
///
/// Holds per-session tunnel state and forwards encrypted requests
/// between the client (browser) and the target node. The gateway sees
/// only ciphertext on the wire.
pub struct RemoteDashboardProxy {
    /// Target node URL or hypermesh domain (e.g. `yourname.hypermesh`,
    /// `host.example:8443`).
    target_node: String,
    /// Opaque session token from the daemon (serialized
    /// `CapabilityToken`); the gateway forwards this as-is and the
    /// target node validates it.
    session_token: Vec<u8>,
    /// E2E tunnel — gateway only ever sees ciphertext through this.
    tunnel: Arc<dyn KyberAesTunnel>,
    /// Operating mode (controls bootstrap / federation behavior).
    mode: DashboardServeMode,
    /// Stable session ID derived from the token bytes (for logging).
    session_id: String,
}

impl RemoteDashboardProxy {
    /// Construct a remote proxy session.
    ///
    /// Uses [`MockTunnel`] until K.2.5 wires the real Kyber→AES path.
    /// The proxy structure (capability-token forwarding, mode routing,
    /// byte-forwarding) is the architectural commitment of K.2.
    pub fn new(
        target: String,
        session_token: Vec<u8>,
        mode: DashboardServeMode,
    ) -> Result<Self, RemoteProxyError> {
        if target.is_empty() {
            return Err(RemoteProxyError::UnreachableTarget { target });
        }
        if session_token.is_empty() {
            return Err(RemoteProxyError::InvalidToken(
                "empty session token".into(),
            ));
        }

        let session_id = hex::encode(&blake3::hash(&session_token).as_bytes()[..8]);
        let tunnel_id = format!("rp-{session_id}");
        let tunnel = Arc::new(MockTunnel::new(tunnel_id, &session_token));

        debug!(
            target = %target,
            mode = mode.as_str(),
            session = %session_id,
            "RemoteDashboardProxy session opened",
        );

        Ok(Self {
            target_node: target,
            session_token,
            tunnel,
            mode,
            session_id,
        })
    }

    /// Construct a remote proxy with a caller-supplied tunnel. Used by
    /// integration tests that want to inject a deterministic tunnel.
    pub fn with_tunnel(
        target: String,
        session_token: Vec<u8>,
        mode: DashboardServeMode,
        tunnel: Arc<dyn KyberAesTunnel>,
    ) -> Result<Self, RemoteProxyError> {
        if target.is_empty() {
            return Err(RemoteProxyError::UnreachableTarget { target });
        }
        if session_token.is_empty() {
            return Err(RemoteProxyError::InvalidToken(
                "empty session token".into(),
            ));
        }
        let session_id = hex::encode(&blake3::hash(&session_token).as_bytes()[..8]);
        Ok(Self {
            target_node: target,
            session_token,
            tunnel,
            mode,
            session_id,
        })
    }

    /// Stable session id (first 8 bytes of `blake3(token)` hex-encoded).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Configured target node.
    pub fn target_node(&self) -> &str {
        &self.target_node
    }

    /// Configured serve mode.
    pub fn mode(&self) -> DashboardServeMode {
        self.mode
    }

    /// True when the underlying tunnel is still alive.
    pub fn is_active(&self) -> bool {
        self.tunnel.is_active()
    }

    /// Forward an HTTP request through the tunnel.
    ///
    /// The request body is sealed with the per-session AES key before
    /// it leaves the gateway. The response is decrypted on receipt.
    /// **Gateway sees ciphertext only** — this is the architectural
    /// guarantee of the remote-proxy mode.
    pub async fn forward_request(
        &self,
        request: ForwardedRequest,
    ) -> Result<ForwardedResponse, RemoteProxyError> {
        if !self.is_active() {
            return Err(RemoteProxyError::SessionClosed);
        }

        // 1. Serialize the plaintext request.
        let plaintext = serde_json::to_vec(&request).map_err(|e| {
            RemoteProxyError::ForwardError(format!("serialize request: {e}"))
        })?;

        // 2. Seal under the session key. Everything below here is
        //    ciphertext from the gateway's POV.
        let sealed = self.tunnel.seal_request(&plaintext).await?;

        // 3. Hand off to wire-transport. K.2 ships the proxy structure
        //    and exercises this path through MockTunnel; K.2.5 wires
        //    the real STOQ wire transport. Until then, the wire
        //    transport is a loopback that decrypts and produces a
        //    canned response (sufficient for proxy-structure tests).
        let response_ciphertext = self.simulate_wire_loopback(&sealed).await?;

        // 4. Decrypt response.
        let response_plain = self.tunnel.open_response(&response_ciphertext).await?;
        let response: ForwardedResponse =
            serde_json::from_slice(&response_plain).map_err(|e| {
                RemoteProxyError::ForwardError(format!("deserialize response: {e}"))
            })?;
        Ok(response)
    }

    /// Tear down the proxy session.
    pub async fn close(&self) {
        self.tunnel.close().await;
        debug!(session = %self.session_id, "RemoteDashboardProxy session closed");
    }

    /// Length of the opaque session token (for callers that want to
    /// audit token forwarding without seeing the bytes).
    pub fn token_byte_len(&self) -> usize {
        self.session_token.len()
    }

    /// Simulated wire transport — for K.2 the gateway is a loopback
    /// that round-trips ciphertext through itself and produces a
    /// deterministic encrypted response. K.2.5 replaces this with the
    /// STOQ wire path to the target node.
    async fn simulate_wire_loopback(
        &self,
        sealed_request: &[u8],
    ) -> Result<Vec<u8>, RemoteProxyError> {
        // Produce a canned response that mirrors the "echo" semantic.
        // The gateway never decrypts the request — it only counts the
        // bytes it forwarded. The canned response is encrypted with
        // the SAME tunnel (in K.2.5 this will instead be encrypted by
        // the target node's tunnel half).
        let canned = ForwardedResponse {
            status: 200,
            body: format!(
                "remote-proxy-loopback target={} bytes={} mode={}",
                self.target_node,
                sealed_request.len(),
                self.mode.as_str(),
            )
            .into_bytes(),
            content_type: "text/plain; charset=utf-8".to_string(),
        };
        let plain = serde_json::to_vec(&canned).map_err(|e| {
            RemoteProxyError::ForwardError(format!("serialize loopback: {e}"))
        })?;
        self.tunnel.seal_request(&plain).await
    }
}

/// Manager for live remote-proxy sessions. Holds proxies by their
/// stable session id so the gateway can route follow-up requests on
/// the same tunnel.
pub struct RemoteProxyManager {
    sessions: Arc<RwLock<HashMap<String, Arc<RemoteDashboardProxy>>>>,
}

impl RemoteProxyManager {
    /// Empty manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new proxy session and return its session id.
    pub async fn register(&self, proxy: Arc<RemoteDashboardProxy>) -> String {
        let sid = proxy.session_id().to_string();
        self.sessions.write().await.insert(sid.clone(), proxy);
        sid
    }

    /// Look up a session by id.
    pub async fn get(&self, sid: &str) -> Option<Arc<RemoteDashboardProxy>> {
        self.sessions.read().await.get(sid).cloned()
    }

    /// Close and remove a session.
    pub async fn close(&self, sid: &str) -> bool {
        let proxy_opt = self.sessions.write().await.remove(sid);
        if let Some(proxy) = proxy_opt {
            proxy.close().await;
            true
        } else {
            false
        }
    }

    /// Number of currently registered sessions.
    pub async fn len(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// True when no sessions are registered.
    pub async fn is_empty(&self) -> bool {
        self.sessions.read().await.is_empty()
    }

    /// Drop expired or inactive sessions. Returns the number reaped.
    pub async fn reap_inactive(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, p| p.is_active());
        let after = sessions.len();
        let reaped = before.saturating_sub(after);
        if reaped > 0 {
            warn!(count = reaped, "reaped inactive remote-proxy sessions");
        }
        reaped
    }
}

impl Default for RemoteProxyManager {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_serve_mode_parse_all_three() {
        assert_eq!(
            DashboardServeMode::parse("trust-proxy"),
            Some(DashboardServeMode::TrustProxy),
        );
        assert_eq!(
            DashboardServeMode::parse("trust_proxy"),
            Some(DashboardServeMode::TrustProxy),
        );
        assert_eq!(
            DashboardServeMode::parse("private-domain"),
            Some(DashboardServeMode::PrivateDomain),
        );
        assert_eq!(
            DashboardServeMode::parse("self-hosted"),
            Some(DashboardServeMode::SelfHosted),
        );
        assert_eq!(DashboardServeMode::parse("garbage"), None);
        assert_eq!(DashboardServeMode::parse(""), None);
    }

    #[test]
    fn dashboard_serve_mode_as_str_round_trip() {
        for m in [
            DashboardServeMode::TrustProxy,
            DashboardServeMode::PrivateDomain,
            DashboardServeMode::SelfHosted,
        ] {
            assert_eq!(DashboardServeMode::parse(m.as_str()), Some(m));
        }
    }

    #[tokio::test]
    async fn mock_tunnel_round_trip() {
        let t = MockTunnel::new("test-tunnel".into(), b"session-token-bytes");
        assert!(t.is_active());
        let pt = b"hello world";
        let ct = t.seal_request(pt).await.expect("test: seal");
        assert_ne!(ct, pt, "ciphertext must differ from plaintext");
        let back = t.open_response(&ct).await.expect("test: open");
        assert_eq!(back, pt);
        t.close().await;
        assert!(!t.is_active());
        assert!(t.seal_request(pt).await.is_err());
    }

    #[tokio::test]
    async fn remote_proxy_construct_and_forward() {
        let proxy = RemoteDashboardProxy::new(
            "yourname.hypermesh".into(),
            b"session-token-12345".to_vec(),
            DashboardServeMode::PrivateDomain,
        )
        .expect("test: construct");
        assert!(proxy.is_active());
        assert_eq!(proxy.target_node(), "yourname.hypermesh");
        assert_eq!(proxy.mode(), DashboardServeMode::PrivateDomain);
        assert_eq!(proxy.token_byte_len(), 19);

        let req = ForwardedRequest {
            method: "GET".into(),
            path: "/dashboard/index.html".into(),
            body: vec![],
        };
        let resp = proxy.forward_request(req).await.expect("test: forward");
        assert_eq!(resp.status, 200);
        assert!(String::from_utf8_lossy(&resp.body).contains("yourname.hypermesh"));
    }

    #[tokio::test]
    async fn remote_proxy_rejects_empty_target() {
        let res = RemoteDashboardProxy::new(
            "".into(),
            b"some-token".to_vec(),
            DashboardServeMode::TrustProxy,
        );
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn remote_proxy_rejects_empty_token() {
        let res = RemoteDashboardProxy::new(
            "node.example".into(),
            vec![],
            DashboardServeMode::TrustProxy,
        );
        let err = res.expect_err("test: empty token must be rejected");
        assert!(matches!(err, RemoteProxyError::InvalidToken(_)));
    }

    #[tokio::test]
    async fn remote_proxy_close_disables_forwarding() {
        let proxy = RemoteDashboardProxy::new(
            "node.example".into(),
            b"tok".to_vec(),
            DashboardServeMode::SelfHosted,
        )
        .expect("test: construct");
        proxy.close().await;
        assert!(!proxy.is_active());
        let req = ForwardedRequest {
            method: "GET".into(),
            path: "/".into(),
            body: vec![],
        };
        let res = proxy.forward_request(req).await;
        assert!(matches!(res, Err(RemoteProxyError::SessionClosed)));
    }

    #[tokio::test]
    async fn remote_proxy_manager_register_get_close() {
        let mgr = RemoteProxyManager::new();
        assert!(mgr.is_empty().await);
        let proxy = Arc::new(
            RemoteDashboardProxy::new(
                "node.example".into(),
                b"tok-A".to_vec(),
                DashboardServeMode::TrustProxy,
            )
            .expect("test: construct"),
        );
        let sid = mgr.register(proxy.clone()).await;
        assert_eq!(mgr.len().await, 1);
        let fetched = mgr.get(&sid).await.expect("present");
        assert!(Arc::ptr_eq(&fetched, &proxy));
        assert!(mgr.close(&sid).await);
        assert_eq!(mgr.len().await, 0);
        assert!(!mgr.close(&sid).await, "closing twice is a no-op");
    }

    #[tokio::test]
    async fn remote_proxy_manager_reap_inactive() {
        let mgr = RemoteProxyManager::new();
        let proxy = Arc::new(
            RemoteDashboardProxy::new(
                "node.example".into(),
                b"tok-B".to_vec(),
                DashboardServeMode::TrustProxy,
            )
            .expect("test: construct"),
        );
        let _sid = mgr.register(proxy.clone()).await;
        proxy.close().await;
        let reaped = mgr.reap_inactive().await;
        assert_eq!(reaped, 1);
        assert!(mgr.is_empty().await);
    }

    #[tokio::test]
    async fn kyber_kem_tunnel_pending_k2_5() {
        // Confirm the production tunnel is intentionally stubbed —
        // K.2 ships the structure, K.2.5 wires the real KEM.
        let res = KyberKemTunnel::new("target", b"token");
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(matches!(err, RemoteProxyError::HandshakeFailed(_)));
        assert!(err.to_string().contains("K.2.5"));
    }

    #[tokio::test]
    async fn remote_proxy_with_tunnel_uses_injected() {
        let tunnel: Arc<dyn KyberAesTunnel> =
            Arc::new(MockTunnel::new("inj".into(), b"injected"));
        let proxy = RemoteDashboardProxy::with_tunnel(
            "node.example".into(),
            b"token".to_vec(),
            DashboardServeMode::SelfHosted,
            tunnel,
        )
        .expect("test: construct");
        assert!(proxy.is_active());
        let req = ForwardedRequest {
            method: "POST".into(),
            path: "/api/echo".into(),
            body: b"payload".to_vec(),
        };
        let resp = proxy.forward_request(req).await.expect("test: forward");
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn remote_proxy_error_maps_to_gateway_error() {
        let invalid: GatewayError = RemoteProxyError::InvalidToken("bad".into()).into();
        assert_eq!(invalid.status_code(), 401);
        let unreach: GatewayError = RemoteProxyError::UnreachableTarget {
            target: "x".into(),
        }
        .into();
        assert_eq!(unreach.status_code(), 404);
        let other: GatewayError = RemoteProxyError::SessionClosed.into();
        assert_eq!(other.status_code(), 502);
    }
}
