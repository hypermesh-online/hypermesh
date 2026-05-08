// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Request dispatch for the IPC server.
//!
//! Handlers are registered by method name and invoked on incoming requests.

use crate::ipc::protocol::{
    RpcError, RpcRequest, RpcResponse, IPC_PROTOCOL_VERSION, METHOD_NOT_FOUND,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Async handler function signature.
///
/// Receives JSON params, returns JSON result or an `RpcError`.
pub type HandlerFn = Arc<
    dyn Fn(
            serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, RpcError>> + Send>>
        + Send
        + Sync,
>;

/// Dispatches incoming RPC requests to registered handlers.
#[derive(Default)]
pub struct RequestHandler {
    handlers: HashMap<String, HandlerFn>,
}

impl std::fmt::Debug for RequestHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestHandler")
            .field("methods", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl RequestHandler {
    /// Create an empty handler registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for the given method name.
    pub fn register(&mut self, method: &str, handler: HandlerFn) {
        self.handlers.insert(method.to_string(), handler);
    }

    /// Return the names of all registered methods.
    pub fn methods(&self) -> Vec<&str> {
        self.handlers.keys().map(|k| k.as_str()).collect()
    }

    /// Dispatch a request to the matching handler, returning an `RpcResponse`.
    pub async fn dispatch(&self, request: RpcRequest) -> RpcResponse {
        match self.handlers.get(&request.method) {
            Some(handler) => match handler(request.params).await {
                Ok(result) => RpcResponse::success(request.id, result),
                Err(rpc_err) => RpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(rpc_err),
                    id: request.id,
                    protocol_version: Some(IPC_PROTOCOL_VERSION.to_string()),
                },
            },
            None => RpcResponse::error(
                request.id,
                METHOD_NOT_FOUND,
                format!("method not found: {}", request.method),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::INVALID_PARAMS;

    #[tokio::test]
    async fn test_dispatch_known_method() {
        let mut handler = RequestHandler::new();
        handler.register(
            "ping",
            Arc::new(|_params| {
                Box::pin(async { Ok(serde_json::json!("pong")) })
            }),
        );

        let req = RpcRequest::new("ping", serde_json::json!(null));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        assert_eq!(
            resp.result.expect("test: result present"),
            serde_json::json!("pong")
        );
    }

    #[tokio::test]
    async fn test_dispatch_unknown_method() {
        let handler = RequestHandler::new();
        let req = RpcRequest::new("nonexistent", serde_json::json!(null));
        let resp = handler.dispatch(req).await;
        assert!(resp.result.is_none());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_dispatch_handler_error() {
        let mut handler = RequestHandler::new();
        handler.register(
            "fail",
            Arc::new(|_params| {
                Box::pin(async {
                    Err(RpcError {
                        code: INVALID_PARAMS,
                        message: "bad params".to_string(),
                        data: None,
                    })
                })
            }),
        );

        let req = RpcRequest::new("fail", serde_json::json!(null));
        let resp = handler.dispatch(req).await;
        assert!(resp.result.is_none());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert_eq!(err.message, "bad params");
    }
}
