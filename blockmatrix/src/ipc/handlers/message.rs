// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Message IPC handlers: send, inbox, history, read.
//!
//! Exposes direct messaging operations over JSON-RPC 2.0.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;

/// Register message-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // message.send
    {
        let s = state.clone();
        handler.register(
            "message.send",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_message_send(params, &s).await })
            }),
        );
    }

    // message.inbox
    {
        let s = state.clone();
        handler.register(
            "message.inbox",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_message_inbox(params, &s).await })
            }),
        );
    }

    // message.history
    {
        let s = state.clone();
        handler.register(
            "message.history",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_message_history(params, &s).await })
            }),
        );
    }

    // message.read
    {
        let s = state.clone();
        handler.register(
            "message.read",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_message_read(params, &s).await })
            }),
        );
    }
}

/// Handle `message.send` -- create and queue a direct message.
///
/// Params:
///   - `recipient` (string, required): Recipient node ID or DNS name
///   - `body` (string, required): Plaintext message body
///   - `content_type` (string, optional): MIME type (default "text/plain")
///   - `reply_to` (string, optional): Parent message ID for threading
async fn handle_message_send(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let recipient = params
        .get("recipient")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'recipient' parameter".into(),
            data: None,
        })?;

    let body = params
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'body' parameter".into(),
            data: None,
        })?;

    let content_type = params
        .get("content_type")
        .and_then(|v| v.as_str())
        .unwrap_or("text/plain");

    let reply_to = params
        .get("reply_to")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Resolve recipient via DNS if it looks like a name
    let resolved_recipient = if recipient.len() < 64 {
        match state.dns_resolver.resolve(recipient).await {
            Some(addr) => format!("{recipient} ({addr})"),
            None => recipient.to_string(),
        }
    } else {
        recipient.to_string()
    };

    let msg = crate::messaging::message::DirectMessage::new(
        state.node_id.clone(),
        None,
        recipient.to_string(),
        content_type.to_string(),
        reply_to,
    );

    let message_id = msg.message_id.clone();

    // For alpha: message created in memory. Encryption + signing + STOQ
    // delivery requires access to Kyber/FALCON keys which are not yet
    // wired into DaemonState. The message structure is ready.

    Ok(serde_json::json!({
        "message_id": message_id,
        "recipient": resolved_recipient,
        "body_length": body.len(),
        "content_type": content_type,
        "status": "created",
    }))
}

/// Handle `message.inbox` -- list received messages.
///
/// Params (all optional):
///   - `limit` (u64): Max messages to return (default 20)
async fn handle_message_inbox(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    // Alpha: return empty inbox (MessageStore not yet in DaemonState).
    let _ = (state, limit);

    Ok(serde_json::json!({
        "messages": [],
        "count": 0,
        "limit": limit,
    }))
}

/// Handle `message.history` -- chat history with a specific peer.
///
/// Params:
///   - `peer` (string, required): Peer node ID
///   - `limit` (u64, optional): Max messages to return (default 50)
async fn handle_message_history(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let peer = params
        .get("peer")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'peer' parameter".into(),
            data: None,
        })?;

    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    // Alpha: return empty history (MessageStore not yet in DaemonState).
    let _ = (state, limit);

    Ok(serde_json::json!({
        "peer": peer,
        "messages": [],
        "count": 0,
        "limit": limit,
    }))
}

/// Handle `message.read` -- retrieve a single message by ID.
///
/// Params:
///   - `message_id` (string, required): ID of the message to read
async fn handle_message_read(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let message_id = params
        .get("message_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'message_id' parameter".into(),
            data: None,
        })?;

    // Alpha: return metadata without decryption (Kyber secret key
    // not yet available in DaemonState).
    let _ = state;

    Ok(serde_json::json!({
        "message_id": message_id,
        "status": "not_found",
        "note": "message store not yet wired to IPC",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::handler::RequestHandler;
    use crate::ipc::protocol::RpcRequest;

    async fn test_state() -> Arc<DaemonState> {
        crate::ipc::handlers::tests::test_state().await
    }

    #[tokio::test]
    async fn test_message_send_params() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // Success
        let req = RpcRequest::new(
            "message.send",
            serde_json::json!({
                "recipient": "peer-xyz",
                "body": "Hello!",
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "expected success: {:?}", resp.error);
        let result = resp.result.expect("test: result present");
        assert_eq!(result["status"], "created");
        assert!(
            result["message_id"]
                .as_str()
                .expect("test: message_id")
                .starts_with("msg-"),
        );

        // Missing recipient
        let req = RpcRequest::new(
            "message.send",
            serde_json::json!({"body": "hi"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        assert!(
            resp.error
                .expect("test: error")
                .message
                .contains("recipient"),
        );

        // Missing body
        let req = RpcRequest::new(
            "message.send",
            serde_json::json!({"recipient": "peer"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        assert!(
            resp.error.expect("test: error").message.contains("body"),
        );
    }

    #[tokio::test]
    async fn test_message_inbox_empty() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("message.inbox", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
        assert!(result["messages"].is_array());
    }

    #[tokio::test]
    async fn test_message_history_params() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // Success
        let req = RpcRequest::new(
            "message.history",
            serde_json::json!({"peer": "alice", "limit": 10}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["peer"], "alice");
        assert_eq!(result["limit"], 10);

        // Missing peer
        let req = RpcRequest::new("message.history", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        assert!(
            resp.error.expect("test: error").message.contains("peer"),
        );
    }

    #[tokio::test]
    async fn test_message_read_params() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // Success
        let req = RpcRequest::new(
            "message.read",
            serde_json::json!({"message_id": "msg-abc123"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["message_id"], "msg-abc123");

        // Missing message_id
        let req = RpcRequest::new("message.read", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        assert!(
            resp.error
                .expect("test: error")
                .message
                .contains("message_id"),
        );
    }
}
