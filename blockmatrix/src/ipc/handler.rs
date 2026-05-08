// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Request dispatch for the IPC server.
//!
//! Handlers are registered by method name and invoked on incoming requests.

use crate::auth::{Capability, CapabilityToken, CapabilityTokenIssuer, RevocationRegistry};
use crate::ipc::handlers::auth::CAPABILITY_DENIED;
use crate::ipc::handlers::capability_registry;
use crate::ipc::protocol::{
    RpcError, RpcRequest, RpcResponse, IPC_PROTOCOL_VERSION, METHOD_NOT_FOUND,
};
use base64::Engine;
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

/// Phase K.2 — capability enforcement context.
///
/// Holds the daemon's token-verification dependencies. Constructed and
/// installed on a [`RequestHandler`] only when
/// `state.capability_token_issuer.is_some()`. When this is `None`,
/// dispatch skips token validation entirely (alpha-default inert).
#[derive(Clone)]
pub struct CapabilityContext {
    /// FALCON-1024 verifier key — daemon's pubkey, the same key the
    /// issuer signs with.
    daemon_pubkey: Vec<u8>,
    /// Revocation registry consulted on every request.
    revocation_registry: Arc<RevocationRegistry>,
}

impl CapabilityContext {
    /// Construct a context from the daemon's issuer + revocation registry.
    pub fn new(
        issuer: &CapabilityTokenIssuer,
        revocation_registry: Arc<RevocationRegistry>,
    ) -> Self {
        Self {
            daemon_pubkey: issuer.daemon_pubkey().to_vec(),
            revocation_registry,
        }
    }

    /// Validate a base64-encoded token against the configured policy.
    ///
    /// Returns `Ok(())` when the token is signed by the daemon, not
    /// expired, not revoked, and grants `required`. Returns
    /// [`CAPABILITY_DENIED`] [`RpcError`] on any failure.
    pub async fn validate(
        &self,
        token_b64: &str,
        required: Capability,
    ) -> Result<(), RpcError> {
        // Decode base64 → JSON bytes → CapabilityToken.
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(token_b64)
            .map_err(|e| RpcError {
                code: CAPABILITY_DENIED,
                message: format!("capability_token base64 decode failed: {e}"),
                data: None,
            })?;
        let token: CapabilityToken =
            serde_json::from_slice(&bytes).map_err(|e| RpcError {
                code: CAPABILITY_DENIED,
                message: format!("capability_token JSON decode failed: {e}"),
                data: None,
            })?;

        // Signature.
        if !token.verify(&self.daemon_pubkey) {
            return Err(RpcError {
                code: CAPABILITY_DENIED,
                message: "capability_token signature verification failed".into(),
                data: None,
            });
        }
        // Expiry.
        if token.is_expired() {
            return Err(RpcError {
                code: CAPABILITY_DENIED,
                message: "capability_token expired".into(),
                data: None,
            });
        }
        // Revocation.
        if self.revocation_registry.is_revoked(&token.session_id).await {
            return Err(RpcError {
                code: CAPABILITY_DENIED,
                message: format!(
                    "capability_token session {} has been revoked",
                    token.session_id
                ),
                data: None,
            });
        }
        // Scope.
        if !token.allows(&required) {
            return Err(RpcError {
                code: CAPABILITY_DENIED,
                message: format!(
                    "capability_token scope {:?} does not grant required {:?}",
                    token.capabilities, required
                ),
                data: None,
            });
        }
        Ok(())
    }
}

/// Dispatches incoming RPC requests to registered handlers.
#[derive(Default)]
pub struct RequestHandler {
    handlers: HashMap<String, HandlerFn>,
    /// Phase K.2 — optional capability enforcement. When `Some`, every
    /// dispatch consults [`capability_registry::required_capability`]
    /// for the method and validates the request's `capability_token`.
    /// When `None`, dispatch skips token enforcement (alpha-default
    /// inert behavior, preserving the localhost-only IPC contract).
    capability_context: Option<CapabilityContext>,
}

impl std::fmt::Debug for RequestHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestHandler")
            .field("methods", &self.handlers.keys().collect::<Vec<_>>())
            .field(
                "capability_enforcement",
                &self.capability_context.is_some(),
            )
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

    /// Phase K.2 — install capability enforcement. After this, every
    /// dispatched request must carry a valid `capability_token` whose
    /// granted scope is at least the method's
    /// [`capability_registry::required_capability`].
    pub fn set_capability_context(&mut self, ctx: CapabilityContext) {
        self.capability_context = Some(ctx);
    }

    /// True when capability enforcement is installed (alpha-default off).
    pub fn capability_enforcement_enabled(&self) -> bool {
        self.capability_context.is_some()
    }

    /// Return the names of all registered methods.
    pub fn methods(&self) -> Vec<&str> {
        self.handlers.keys().map(|k| k.as_str()).collect()
    }

    /// Dispatch a request to the matching handler, returning an `RpcResponse`.
    ///
    /// When capability enforcement is enabled, the request's
    /// `capability_token` is validated *before* the handler is invoked.
    pub async fn dispatch(&self, request: RpcRequest) -> RpcResponse {
        // Phase K.2 — capability enforcement (alpha-default inert).
        if let Some(ctx) = self.capability_context.as_ref() {
            if !capability_registry::always_public(&request.method) {
                let required = capability_registry::required_capability(&request.method);
                let token_b64 = match request.capability_token.as_deref() {
                    Some(t) => t,
                    None => {
                        return RpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(RpcError {
                                code: CAPABILITY_DENIED,
                                message: format!(
                                    "method '{}' requires capability {:?} but no \
                                     capability_token was supplied",
                                    request.method, required
                                ),
                                data: None,
                            }),
                            id: request.id,
                            protocol_version: Some(IPC_PROTOCOL_VERSION.to_string()),
                        };
                    }
                };
                if let Err(rpc_err) = ctx.validate(token_b64, required).await {
                    return RpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(rpc_err),
                        id: request.id,
                        protocol_version: Some(IPC_PROTOCOL_VERSION.to_string()),
                    };
                }
            }
        }

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
