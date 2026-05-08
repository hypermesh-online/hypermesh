// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase K.2 — remote dashboard proxy.
//!
//! Given a target node URL plus a valid capability token, proxies
//! dashboard HTTP/3 traffic from a client browser to the target node
//! over the **existing STOQ stream** between gateway and target.
//!
//! ## Why no bespoke crypto here
//!
//! STOQ already provides transport security on every peer connection:
//!
//! * **Wire encryption**: X25519MLKEM768 hybrid post-quantum QUIC key
//!   exchange (negotiated at QUIC handshake, end-to-end between gateway
//!   and target).
//! * **Authentication**: FALCON-1024 signed certs + bilateral PoS
//!   handshake authenticate the tunnel.
//!
//! Kyber-1024 KEM + AES-256-GCM is the **asset-at-rest** primitive
//! (R7) used by the asset pipeline (Compress → Encrypt → Shard →
//! Distribute) for content stored in shards. Picking it here would be
//! both redundant (STOQ already encrypts the wire) and the wrong
//! primitive (transport vs at-rest).
//!
//! The gateway is therefore a **byte-forwarder over an already-secure
//! STOQ stream**. The capability token rides as part of the forwarded
//! request payload; the target node validates it via the same
//! [`crate::auth::CapabilityTokenIssuer`] flow that handles direct
//! daemon IPC.
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
//! ## STOQ wiring status
//!
//! K.2 ships the proxy structure and the capability-token forwarding
//! contract. The wire path is abstracted behind
//! [`StoqStreamForwarder`]; gateway initialization wires a real
//! forwarder over `stoq::transport::connection::Connection::open_stream`
//! once the gateway is configured with a `NetworkManager` handle. Tests
//! and alpha builds run against [`MockStreamForwarder`].

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
    /// Token-validation rejection at the target — the capability token
    /// is invalid, expired, revoked, or doesn't grant the required
    /// scope.
    #[error("capability token rejected: {0}")]
    InvalidToken(String),
    /// Forwarding-time error — STOQ stream open/read/write failure,
    /// payload deserialization, etc.
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
    /// Default convenience tier.
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

/// Minimal HTTP request shape forwarded over a STOQ stream.
///
/// The capability token is embedded so the target node can validate
/// without an out-of-band channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedRequest {
    /// HTTP method, e.g. "GET".
    pub method: String,
    /// Request path including any query string, e.g. "/index.html?v=1".
    pub path: String,
    /// Optional body bytes.
    pub body: Vec<u8>,
}

/// Minimal HTTP response shape returned through the STOQ stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardedResponse {
    /// HTTP status code.
    pub status: u16,
    /// Body bytes.
    pub body: Vec<u8>,
    /// Detected content-type (for browser rendering).
    pub content_type: String,
}

/// Abstracts the STOQ stream open/write/read cycle the proxy needs.
///
/// A production implementation wraps `Arc<NetworkManager>` (or a
/// per-connection `stoq::transport::connection::Connection`) and:
///
/// 1. Looks up (or opens) a STOQ connection to `target_node`.
/// 2. Calls `Connection::open_stream()` for a fresh bi-directional
///    stream.
/// 3. Writes `request_bytes` (the gateway-internal envelope: capability
///    token + serialized [`ForwardedRequest`]) onto the stream.
/// 4. Reads the target's response bytes back from the same stream.
///
/// The wire is already encrypted (QUIC/X25519MLKEM768) and authenticated
/// (FALCON-PoS handshake) — no additional crypto here.
///
/// [`MockStreamForwarder`] provides a deterministic in-memory loop for
/// tests and alpha builds where a `NetworkManager` is not yet wired
/// into the gateway.
#[async_trait]
pub trait StoqStreamForwarder: Send + Sync {
    /// Open a STOQ stream to `target_node`, write `request_bytes`, read
    /// the response bytes back. Returns the response payload.
    async fn forward(
        &self,
        target_node: &str,
        request_bytes: &[u8],
    ) -> Result<Vec<u8>, RemoteProxyError>;
}

/// Deterministic in-memory forwarder for tests and alpha builds.
///
/// Records the most-recent request bytes and target so tests can assert
/// the gateway forwarded what it claims to have forwarded. Returns a
/// canned response that includes the target name and the byte count of
/// the forwarded request — sufficient to exercise the proxy plumbing
/// without standing up a real STOQ peer.
pub struct MockStreamForwarder {
    last: Arc<RwLock<Option<(String, Vec<u8>)>>>,
}

impl MockStreamForwarder {
    /// Construct an empty mock forwarder.
    pub fn new() -> Self {
        Self {
            last: Arc::new(RwLock::new(None)),
        }
    }

    /// Snapshot the most-recent (target, request_bytes) pair, if any.
    pub async fn last_forwarded(&self) -> Option<(String, Vec<u8>)> {
        self.last.read().await.clone()
    }
}

impl Default for MockStreamForwarder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StoqStreamForwarder for MockStreamForwarder {
    async fn forward(
        &self,
        target_node: &str,
        request_bytes: &[u8],
    ) -> Result<Vec<u8>, RemoteProxyError> {
        *self.last.write().await = Some((target_node.to_string(), request_bytes.to_vec()));
        let canned = ForwardedResponse {
            status: 200,
            body: format!(
                "remote-proxy-mock target={} bytes={}",
                target_node,
                request_bytes.len(),
            )
            .into_bytes(),
            content_type: "text/plain; charset=utf-8".to_string(),
        };
        serde_json::to_vec(&canned).map_err(|e| {
            RemoteProxyError::ForwardError(format!("serialize mock response: {e}"))
        })
    }
}

/// Internal envelope sent over the STOQ stream.
///
/// Combines the capability token (so the target can authenticate) with
/// the forwarded HTTP request. Serialized with `serde_json` for now;
/// can be swapped to a more compact format later without affecting
/// callers.
#[derive(Debug, Clone, Serialize)]
struct ProxyEnvelope<'a> {
    capability_token_hex: String,
    request: &'a ForwardedRequest,
}

/// Remote dashboard proxy.
///
/// Holds per-session capability-token + target binding and forwards
/// HTTP requests over an existing STOQ stream via
/// [`StoqStreamForwarder`]. The gateway provides no transport-layer
/// crypto of its own — STOQ encrypts the wire end-to-end.
pub struct RemoteDashboardProxy {
    /// Target node URL or hypermesh domain (e.g. `yourname.hypermesh`,
    /// `host.example:8443`).
    target_node: String,
    /// Opaque capability token from the daemon (serialized
    /// `CapabilityToken`); the gateway forwards this as-is and the
    /// target node validates it.
    capability_token: Vec<u8>,
    /// Operating mode (controls bootstrap / federation behavior).
    mode: DashboardServeMode,
    /// Stable session ID derived from the token bytes (for logging).
    session_id: String,
    /// STOQ stream forwarder — opens streams to `target_node` and
    /// round-trips request/response bytes. Production wiring uses
    /// `NetworkManager`; tests use [`MockStreamForwarder`].
    forwarder: Arc<dyn StoqStreamForwarder>,
    /// Active flag — set to false on [`close`].
    active: Arc<std::sync::atomic::AtomicBool>,
}

impl RemoteDashboardProxy {
    /// Construct a remote proxy session bound to a forwarder.
    pub fn new(
        target: String,
        capability_token: Vec<u8>,
        mode: DashboardServeMode,
        forwarder: Arc<dyn StoqStreamForwarder>,
    ) -> Result<Self, RemoteProxyError> {
        if target.is_empty() {
            return Err(RemoteProxyError::UnreachableTarget { target });
        }
        if capability_token.is_empty() {
            return Err(RemoteProxyError::InvalidToken(
                "empty capability token".into(),
            ));
        }

        let session_id = hex::encode(&blake3::hash(&capability_token).as_bytes()[..8]);

        debug!(
            target = %target,
            mode = mode.as_str(),
            session = %session_id,
            "RemoteDashboardProxy session opened",
        );

        Ok(Self {
            target_node: target,
            capability_token,
            mode,
            session_id,
            forwarder,
            active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
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

    /// True when the session is still alive (has not been closed).
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Length of the opaque capability token (for callers that want to
    /// audit token forwarding without seeing the bytes).
    pub fn token_byte_len(&self) -> usize {
        self.capability_token.len()
    }

    /// Forward an HTTP request to the target node over a STOQ stream.
    ///
    /// The capability token is embedded in the envelope so the target
    /// can authenticate; STOQ handles wire encryption end-to-end.
    pub async fn forward_request(
        &self,
        request: ForwardedRequest,
    ) -> Result<ForwardedResponse, RemoteProxyError> {
        if !self.is_active() {
            return Err(RemoteProxyError::SessionClosed);
        }

        // Serialize the envelope (capability token + request).
        let envelope = ProxyEnvelope {
            capability_token_hex: hex::encode(&self.capability_token),
            request: &request,
        };
        let request_bytes = serde_json::to_vec(&envelope).map_err(|e| {
            RemoteProxyError::ForwardError(format!("serialize envelope: {e}"))
        })?;

        // Hand off to STOQ. Wire is encrypted by QUIC; authentication
        // happens at the target via the capability token in the body.
        let response_bytes = self
            .forwarder
            .forward(&self.target_node, &request_bytes)
            .await?;

        // Parse the response.
        let response: ForwardedResponse =
            serde_json::from_slice(&response_bytes).map_err(|e| {
                RemoteProxyError::ForwardError(format!("deserialize response: {e}"))
            })?;
        Ok(response)
    }

    /// Tear down the proxy session.
    pub async fn close(&self) {
        self.active
            .store(false, std::sync::atomic::Ordering::SeqCst);
        debug!(session = %self.session_id, "RemoteDashboardProxy session closed");
    }
}

/// Manager for live remote-proxy sessions. Holds proxies by their
/// stable session id so the gateway can route follow-up requests to
/// the same target.
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
    async fn remote_proxy_construct_and_forward() {
        let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
        let proxy = RemoteDashboardProxy::new(
            "yourname.hypermesh".into(),
            b"capability-token-12345".to_vec(),
            DashboardServeMode::PrivateDomain,
            forwarder,
        )
        .expect("test: construct");
        assert!(proxy.is_active());
        assert_eq!(proxy.target_node(), "yourname.hypermesh");
        assert_eq!(proxy.mode(), DashboardServeMode::PrivateDomain);
        assert_eq!(proxy.token_byte_len(), 22);

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
        let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
        let res = RemoteDashboardProxy::new(
            "".into(),
            b"some-token".to_vec(),
            DashboardServeMode::TrustProxy,
            forwarder,
        );
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn remote_proxy_rejects_empty_token() {
        let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
        let res = RemoteDashboardProxy::new(
            "node.example".into(),
            vec![],
            DashboardServeMode::TrustProxy,
            forwarder,
        );
        let err = res.expect_err("test: empty token must be rejected");
        assert!(matches!(err, RemoteProxyError::InvalidToken(_)));
    }

    #[tokio::test]
    async fn remote_proxy_close_disables_forwarding() {
        let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
        let proxy = RemoteDashboardProxy::new(
            "node.example".into(),
            b"tok".to_vec(),
            DashboardServeMode::SelfHosted,
            forwarder,
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
        let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
        let proxy = Arc::new(
            RemoteDashboardProxy::new(
                "node.example".into(),
                b"tok-A".to_vec(),
                DashboardServeMode::TrustProxy,
                forwarder,
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
        let forwarder: Arc<dyn StoqStreamForwarder> = Arc::new(MockStreamForwarder::new());
        let proxy = Arc::new(
            RemoteDashboardProxy::new(
                "node.example".into(),
                b"tok-B".to_vec(),
                DashboardServeMode::TrustProxy,
                forwarder,
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
    async fn mock_forwarder_records_request_bytes() {
        let mock = Arc::new(MockStreamForwarder::new());
        let forwarder: Arc<dyn StoqStreamForwarder> = mock.clone();
        let proxy = RemoteDashboardProxy::new(
            "trust.hypermesh.online".into(),
            b"capability-XYZ".to_vec(),
            DashboardServeMode::TrustProxy,
            forwarder,
        )
        .expect("test: construct");
        let req = ForwardedRequest {
            method: "POST".into(),
            path: "/api/echo".into(),
            body: b"payload".to_vec(),
        };
        let _resp = proxy.forward_request(req).await.expect("test: forward");
        let (target, bytes) = mock.last_forwarded().await.expect("test: recorded");
        assert_eq!(target, "trust.hypermesh.online");
        // Capability token must appear in the forwarded envelope so
        // the target can authenticate.
        let envelope_str = String::from_utf8_lossy(&bytes);
        assert!(envelope_str.contains(&hex::encode(b"capability-XYZ")));
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
