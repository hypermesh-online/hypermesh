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

/// JSON-RPC 2.0 version string.
const JSONRPC_VERSION: &str = "2.0";

/// Global request ID counter for client-side auto-increment.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

impl RpcRequest {
    /// Create a new request with an auto-incrementing ID.
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: u64,
}

impl RpcResponse {
    /// Construct a success response.
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
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
}
