// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! JSON-RPC 2.0 protocol types and wire format for HyperMesh IPC.
//!
//! Wire protocol: newline-delimited JSON (`<json>\n`) over Unix domain sockets.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

// JSON-RPC 2.0 standard error codes
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

// HyperMesh-specific error codes
pub const DAEMON_NOT_RUNNING: i64 = -32000;

/// Phase K.2 — capability denied. Mirror of
/// [`crate::ipc::handlers::auth::CAPABILITY_DENIED`] re-exported here so
/// callers and tests don't have to reach into the auth handler module.
pub const CAPABILITY_DENIED: i64 = -32004;

/// Phase J.1 — IPC protocol version mismatch (CLI/daemon major version
/// disagree). The error message hints the caller to upgrade.
pub const PROTOCOL_VERSION_MISMATCH: i64 = -32100;

/// JSON-RPC 2.0 version string.
const JSONRPC_VERSION: &str = "2.0";

/// Phase J.1 — current IPC protocol version (semver-tracked; tied to
/// `CARGO_PKG_VERSION`). Major-version mismatch between CLI and daemon
/// is rejected with [`PROTOCOL_VERSION_MISMATCH`]; minor-version
/// mismatch is forward-compatible.
pub const IPC_PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Parse the major-version component of a semver string, returning
/// `None` on parse failure (treated as 0 by the comparator).
pub fn major_version(version: &str) -> Option<u64> {
    let core = version.split('-').next().unwrap_or(version);
    core.split('.').next()?.parse::<u64>().ok()
}

/// True when `client_version` and `daemon_version` share the same
/// major-version component (or both are unparseable, treated as 0).
pub fn protocol_versions_compatible(client_version: &str, daemon_version: &str) -> bool {
    major_version(client_version).unwrap_or(0)
        == major_version(daemon_version).unwrap_or(0)
}

/// Global request ID counter for client-side auto-increment.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// JSON-RPC 2.0 request.
///
/// Phase J.1: the optional `protocol_version` field carries the
/// caller's `CARGO_PKG_VERSION`. Daemons reject requests with a
/// different major version. Old clients that don't send the field
/// (`#[serde(default)]`) are accepted (forward-compat with v0 nodes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
    /// Caller's IPC protocol version (semver).  Optional for
    /// backwards-compat with pre-J.1 clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    /// Phase K.2 — optional capability token (base64 of serialized
    /// `CapabilityToken`). When the daemon is configured for token
    /// enforcement (`state.capability_token_issuer.is_some()`),
    /// requests without a token, or with insufficient scope, are
    /// rejected with [`CAPABILITY_DENIED`]. Pre-K.2 clients that omit
    /// the field are accepted only when enforcement is not configured
    /// (alpha-default inert behavior).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_token: Option<String>,
}

impl RpcRequest {
    /// Create a new request with an auto-incrementing ID, embedding
    /// the current `IPC_PROTOCOL_VERSION` so the daemon can detect
    /// major-version mismatches.
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            protocol_version: Some(IPC_PROTOCOL_VERSION.to_string()),
            capability_token: None,
        }
    }

    /// Phase K.2 — variant of [`RpcRequest::new`] that includes a base64
    /// capability token. Used by SDKs that have a token from
    /// `auth.create_session`.
    pub fn new_with_token(
        method: &str,
        params: serde_json::Value,
        capability_token: String,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            protocol_version: Some(IPC_PROTOCOL_VERSION.to_string()),
            capability_token: Some(capability_token),
        }
    }

    /// Variant of [`RpcRequest::new`] that omits the protocol version.
    /// Test-only; production callers always identify themselves so the
    /// daemon can produce useful upgrade hints.
    #[cfg(test)]
    pub fn new_without_protocol_version(method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            protocol_version: None,
            capability_token: None,
        }
    }
}

/// JSON-RPC 2.0 response.
///
/// Phase J.1: the daemon stamps every response with the daemon's
/// `IPC_PROTOCOL_VERSION` so newer clients can decide whether to
/// upgrade or downgrade behavior locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: u64,
    /// Daemon's IPC protocol version (semver). Always populated by
    /// the daemon; old daemons that don't set it appear as `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
}

impl RpcResponse {
    /// Construct a success response.
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
            protocol_version: Some(IPC_PROTOCOL_VERSION.to_string()),
        }
    }

    /// Construct an error response.
    pub fn error(id: u64, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
            protocol_version: Some(IPC_PROTOCOL_VERSION.to_string()),
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Resolve the Unix socket path for the daemon control channel.
///
/// Precedence:
/// 1. `$HYPERMESH_SOCK` environment variable
/// 2. `$XDG_RUNTIME_DIR/hypermesh/ctl.sock`
/// 3. `~/.hypermesh/ctl.sock`
pub fn socket_path() -> PathBuf {
    if let Ok(path) = std::env::var("HYPERMESH_SOCK") {
        return PathBuf::from(path);
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir)
            .join("hypermesh")
            .join("ctl.sock");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".hypermesh")
        .join("ctl.sock")
}

/// Resolve the PID file path (sibling to the socket).
pub fn pid_file_path() -> PathBuf {
    let mut path = socket_path();
    path.set_file_name("daemon.pid");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_round_trip() {
        let req = RpcRequest::new("status", serde_json::json!({"verbose": true}));
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "status");

        let serialized = serde_json::to_string(&req).expect("test: serialize");
        let deserialized: RpcRequest =
            serde_json::from_str(&serialized).expect("test: deserialize");
        assert_eq!(deserialized.method, "status");
        assert_eq!(deserialized.id, req.id);
    }

    #[test]
    fn test_response_success_round_trip() {
        let resp = RpcResponse::success(42, serde_json::json!({"ok": true}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        assert_eq!(resp.id, 42);

        let serialized = serde_json::to_string(&resp).expect("test: serialize");
        // error field should be absent (skip_serializing_if)
        assert!(!serialized.contains("\"error\""));
        let deserialized: RpcResponse =
            serde_json::from_str(&serialized).expect("test: deserialize");
        assert!(deserialized.result.is_some());
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn test_response_error_round_trip() {
        let resp = RpcResponse::error(7, METHOD_NOT_FOUND, "no such method");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        let err = resp.error.as_ref().expect("test: error present");
        assert_eq!(err.code, METHOD_NOT_FOUND);
        assert_eq!(err.message, "no such method");

        let serialized = serde_json::to_string(&resp).expect("test: serialize");
        assert!(!serialized.contains("\"result\""));
        let deserialized: RpcResponse =
            serde_json::from_str(&serialized).expect("test: deserialize");
        assert!(deserialized.error.is_some());
    }

    #[test]
    fn test_socket_path_returns_valid_path() {
        // Clear env to test fallback
        let original = std::env::var("HYPERMESH_SOCK").ok();
        std::env::remove_var("HYPERMESH_SOCK");

        let path = socket_path();
        assert!(path.to_str().is_some());
        assert!(path.ends_with("ctl.sock"));

        // Restore
        if let Some(val) = original {
            std::env::set_var("HYPERMESH_SOCK", val);
        }
    }

    #[test]
    fn test_socket_path_env_override() {
        let custom = "/tmp/test-hypermesh.sock";
        std::env::set_var("HYPERMESH_SOCK", custom);
        let path = socket_path();
        assert_eq!(path, PathBuf::from(custom));
        std::env::remove_var("HYPERMESH_SOCK");
    }

    #[test]
    fn test_protocol_versions_same_major_compatible() {
        assert!(protocol_versions_compatible("0.1.0", "0.2.0"));
        assert!(protocol_versions_compatible("1.5.0", "1.0.0"));
        assert!(protocol_versions_compatible("2.0.0", "2.99.0"));
    }

    #[test]
    fn test_protocol_versions_different_major_incompatible() {
        assert!(!protocol_versions_compatible("1.0.0", "2.0.0"));
        assert!(!protocol_versions_compatible("0.5.0", "1.0.0"));
    }

    #[test]
    fn test_request_carries_protocol_version() {
        let req = RpcRequest::new("ping", serde_json::json!(null));
        assert_eq!(req.protocol_version, Some(IPC_PROTOCOL_VERSION.to_string()));
    }

    #[test]
    fn test_request_without_protocol_version_round_trips() {
        let req = RpcRequest::new_without_protocol_version("ping", serde_json::json!(null));
        let s = serde_json::to_string(&req).expect("test: serialize");
        // Old-client compatibility: field is omitted, not null.
        assert!(!s.contains("protocol_version"));
        let back: RpcRequest = serde_json::from_str(&s).expect("test: deserialize");
        assert!(back.protocol_version.is_none());
    }

    #[test]
    fn test_response_carries_protocol_version() {
        let resp = RpcResponse::success(1, serde_json::json!(true));
        assert_eq!(
            resp.protocol_version,
            Some(IPC_PROTOCOL_VERSION.to_string())
        );
    }

    #[test]
    fn test_major_version_parses() {
        assert_eq!(major_version("0.1.0"), Some(0));
        assert_eq!(major_version("1.5.3-rc1"), Some(1));
        assert_eq!(major_version("not-semver"), None);
    }
}
