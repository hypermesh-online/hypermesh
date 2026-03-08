// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! STOQ API bridge for IPC handlers.
//!
//! Exposes all `RequestHandler` methods as STOQ API handlers so that clients
//! can invoke them over the STOQ protocol instead of HTTP REST.
//!
//! IPC method names use dot notation (`blockchain.height`), while STOQ API
//! paths use slash notation (`blockmatrix/blockchain/height`). This module
//! handles the translation transparently.

use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

use stoq::api::{ApiError, ApiHandler, ApiRequest, ApiResponse, StoqApiServer};

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::RpcRequest;

/// Service name used as the first path segment in STOQ API routing.
const SERVICE_NAME: &str = "blockmatrix";

/// Bridge adapter that exposes a single IPC handler method as a STOQ API handler.
///
/// Maps STOQ paths like `blockmatrix/blockchain/height` to IPC methods like
/// `blockchain.height` by converting slashes to dots in the method portion.
pub struct IpcBridgeHandler {
    /// The IPC method name (e.g., `blockchain.height`).
    ipc_method: String,
    /// The STOQ API path (e.g., `blockmatrix/blockchain/height`).
    stoq_path: String,
    /// Shared reference to the IPC request handler.
    handler: Arc<RequestHandler>,
}

impl IpcBridgeHandler {
    /// Create a bridge handler for one IPC method.
    ///
    /// `ipc_method` is the dot-separated IPC method name (e.g. `blockchain.height`).
    /// The STOQ path is derived as `blockmatrix/<method with dots replaced by slashes>`.
    fn new(ipc_method: &str, handler: Arc<RequestHandler>) -> Self {
        let stoq_suffix = ipc_method.replace('.', "/");
        let stoq_path = format!("{SERVICE_NAME}/{stoq_suffix}");
        Self {
            ipc_method: ipc_method.to_string(),
            stoq_path,
            handler,
        }
    }
}

#[async_trait]
impl ApiHandler for IpcBridgeHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        // Deserialize the STOQ payload as JSON params
        let params: serde_json::Value = if request.payload.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&request.payload)
                .map_err(|e| ApiError::InvalidRequest(format!("invalid JSON params: {e}")))?
        };

        debug!(
            "IPC bridge: {} -> {}",
            self.stoq_path, self.ipc_method
        );

        // Build an RPC request and dispatch through the IPC handler
        let rpc_request = RpcRequest::new(&self.ipc_method, params);
        let rpc_response = self.handler.dispatch(rpc_request).await;

        // Translate the RPC response into a STOQ API response
        if let Some(rpc_err) = rpc_response.error {
            Ok(ApiResponse {
                request_id: request.id,
                success: false,
                payload: Bytes::new(),
                error: Some(rpc_err.message),
                metadata: HashMap::new(),
            })
        } else {
            let result = rpc_response.result.unwrap_or(serde_json::Value::Null);
            let payload = serde_json::to_vec(&result)
                .map_err(|e| ApiError::SerializationError(e.to_string()))?;
            Ok(ApiResponse {
                request_id: request.id,
                success: true,
                payload: Bytes::from(payload),
                error: None,
                metadata: HashMap::new(),
            })
        }
    }

    fn path(&self) -> &str {
        &self.stoq_path
    }
}

/// Register all IPC handler methods as STOQ API handlers on the given server.
///
/// Iterates over every method registered in `handler` and creates a
/// corresponding `IpcBridgeHandler` that translates STOQ requests to IPC
/// dispatch calls.
pub fn register_ipc_bridge(api_server: &StoqApiServer, handler: Arc<RequestHandler>) {
    let methods = handler.methods();
    let count = methods.len();

    for method in methods {
        let bridge = IpcBridgeHandler::new(method, handler.clone());
        debug!("Bridging IPC method '{}' -> STOQ '{}'", method, bridge.stoq_path);
        api_server.register_handler(Arc::new(bridge));
    }

    info!(
        "Registered {} IPC method(s) as STOQ API handlers under '{}'",
        count, SERVICE_NAME,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler() -> Arc<RequestHandler> {
        let mut h = RequestHandler::new();
        h.register(
            "ping",
            Arc::new(|_params| {
                Box::pin(async { Ok(serde_json::json!("pong")) })
            }),
        );
        h.register(
            "blockchain.height",
            Arc::new(|_params| {
                Box::pin(async { Ok(serde_json::json!(42)) })
            }),
        );
        h.register(
            "dns.resolve",
            Arc::new(|params| {
                Box::pin(async move {
                    let name = params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    Ok(serde_json::json!({"name": name, "address": "::1"}))
                })
            }),
        );
        Arc::new(h)
    }

    #[test]
    fn test_stoq_path_derivation() {
        let handler = test_handler();
        let bridge = IpcBridgeHandler::new("blockchain.height", handler);
        assert_eq!(bridge.stoq_path, "blockmatrix/blockchain/height");
        assert_eq!(bridge.path(), "blockmatrix/blockchain/height");
    }

    #[test]
    fn test_simple_method_path() {
        let handler = test_handler();
        let bridge = IpcBridgeHandler::new("ping", handler);
        assert_eq!(bridge.stoq_path, "blockmatrix/ping");
    }

    #[tokio::test]
    async fn test_bridge_handle_success() {
        let handler = test_handler();
        let bridge = IpcBridgeHandler::new("ping", handler);
        let request = ApiRequest {
            id: "req-1".to_string(),
            service: "blockmatrix".to_string(),
            method: "ping".to_string(),
            payload: Bytes::new(),
            metadata: HashMap::new(),
        };
        let response = bridge.handle(request).await.expect("test: handle ok");
        assert!(response.success);
        assert_eq!(response.request_id, "req-1");
        let result: serde_json::Value =
            serde_json::from_slice(&response.payload).expect("test: parse payload");
        assert_eq!(result, serde_json::json!("pong"));
    }

    #[tokio::test]
    async fn test_bridge_handle_with_params() {
        let handler = test_handler();
        let bridge = IpcBridgeHandler::new("dns.resolve", handler);
        let payload = serde_json::to_vec(&serde_json::json!({"name": "test.local"}))
            .expect("test: serialize");
        let request = ApiRequest {
            id: "req-2".to_string(),
            service: "blockmatrix".to_string(),
            method: "dns/resolve".to_string(),
            payload: Bytes::from(payload),
            metadata: HashMap::new(),
        };
        let response = bridge.handle(request).await.expect("test: handle ok");
        assert!(response.success);
        let result: serde_json::Value =
            serde_json::from_slice(&response.payload).expect("test: parse payload");
        assert_eq!(result["name"], "test.local");
        assert_eq!(result["address"], "::1");
    }

    #[tokio::test]
    async fn test_bridge_handle_unknown_method() {
        let handler = test_handler();
        let bridge = IpcBridgeHandler::new("nonexistent.method", handler);
        let request = ApiRequest {
            id: "req-3".to_string(),
            service: "blockmatrix".to_string(),
            method: "nonexistent/method".to_string(),
            payload: Bytes::new(),
            metadata: HashMap::new(),
        };
        let response = bridge.handle(request).await.expect("test: handle ok");
        assert!(!response.success);
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_bridge_handle_invalid_json() {
        let handler = test_handler();
        let bridge = IpcBridgeHandler::new("ping", handler);
        let request = ApiRequest {
            id: "req-4".to_string(),
            service: "blockmatrix".to_string(),
            method: "ping".to_string(),
            payload: Bytes::from_static(b"{invalid json"),
            metadata: HashMap::new(),
        };
        let result = bridge.handle(request).await;
        assert!(result.is_err());
    }
}
