// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Share IPC handlers: send, inbox, accept, reject.
//!
//! Exposes P2P file-sharing operations over JSON-RPC 2.0.
//! These handlers are structured to wire into the sharing module (J2)
//! once it is available. Until then they return structured responses
//! indicating what the operation would do.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;

/// Register share-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // share.send -- create and send a share invite to a peer
    {
        let s = state.clone();
        handler.register(
            "share.send",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_share_send(params, &s).await })
            }),
        );
    }

    // share.inbox -- list pending received invites
    {
        let s = state.clone();
        handler.register(
            "share.inbox",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_share_inbox(params, &s).await })
            }),
        );
    }

    // share.accept -- accept a received share invite
    {
        handler.register(
            "share.accept",
            Arc::new(move |params| {
                Box::pin(async move { handle_share_accept(params).await })
            }),
        );
    }

    // share.reject -- reject/dismiss a received share invite
    {
        handler.register(
            "share.reject",
            Arc::new(move |params| {
                Box::pin(async move { handle_share_reject(params).await })
            }),
        );
    }
}

/// Handle `share.send` -- create a share invite for a peer.
///
/// Params:
///   - `asset_id` (string, required): Asset to share
///   - `recipient` (string, required): Recipient node ID or DNS name
async fn handle_share_send(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let asset_id = params
        .get("asset_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'asset_id' parameter".into(),
            data: None,
        })?;

    let recipient = params
        .get("recipient")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'recipient' parameter".into(),
            data: None,
        })?;

    // Resolve recipient via DNS if it looks like a name rather than a node_id
    let resolved_recipient = if recipient.len() < 64 {
        // Short string -- try DNS resolution
        match state.dns_resolver.resolve(recipient).await {
            Some(addr) => format!("{recipient} ({addr})"),
            None => recipient.to_string(),
        }
    } else {
        recipient.to_string()
    };

    // Generate a deterministic invite ID from asset + recipient + timestamp
    let invite_id = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(asset_id.as_bytes());
        hasher.update(recipient.as_bytes());
        hasher.update(&state.started_at.elapsed().as_nanos().to_le_bytes());
        let hash = hasher.finalize();
        format!("inv-{}", &hash.to_hex()[..16])
    };

    // TODO(J2): Once sharing module is wired, this will:
    // 1. Look up the asset's shard map from the blockchain
    // 2. Wrap the decryption key with the recipient's Kyber pubkey
    // 3. Build a ShareInvite and send it via STOQ
    // For alpha: return structured response showing the invite was created.

    Ok(serde_json::json!({
        "invite_id": invite_id,
        "asset_id": asset_id,
        "recipient": resolved_recipient,
        "sender": state.node_id,
        "status": "created",
        "note": "sharing module pending (J2) -- invite structure ready",
    }))
}

/// Handle `share.inbox` -- list pending received share invites.
///
/// Params (all optional):
///   - `limit` (u64): Max invites to return (default 50)
async fn handle_share_inbox(
    params: serde_json::Value,
    _state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50);

    // TODO(J2): Once InboxStore is available, query it for pending invites.
    // For alpha: return empty inbox.

    Ok(serde_json::json!({
        "invites": [],
        "count": 0,
        "limit": limit,
        "note": "inbox store pending (J2)",
    }))
}

/// Handle `share.accept` -- accept a received share invite.
///
/// Params:
///   - `invite_id` (string, required): ID of the invite to accept
async fn handle_share_accept(
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let invite_id = params
        .get("invite_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'invite_id' parameter".into(),
            data: None,
        })?;

    // TODO(J2): Once InboxStore is available:
    // 1. Look up invite by ID
    // 2. Unwrap the Kyber-encrypted decryption key
    // 3. Save the shard map locally
    // 4. Begin shard reconstruction
    // For alpha: acknowledge acceptance.

    Ok(serde_json::json!({
        "invite_id": invite_id,
        "status": "accepted",
        "note": "sharing module pending (J2) -- accept acknowledged",
    }))
}

/// Handle `share.reject` -- reject/dismiss a received share invite.
///
/// Params:
///   - `invite_id` (string, required): ID of the invite to reject
async fn handle_share_reject(
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let invite_id = params
        .get("invite_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "missing or invalid 'invite_id' parameter".into(),
            data: None,
        })?;

    // TODO(J2): Once InboxStore is available, mark invite as rejected.

    Ok(serde_json::json!({
        "invite_id": invite_id,
        "status": "rejected",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::node_chain::NodeBlockchain;
    use crate::bootstrap::DnsResolver;
    use crate::ipc::handler::RequestHandler;
    use crate::ipc::protocol::RpcRequest;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::network::shard_store::ShardStore;
    use crate::persistence::{PersistenceConfig, PersistenceManager};
    use std::path::PathBuf;
    use std::time::Instant;

    async fn test_state() -> Arc<DaemonState> {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: PathBuf::from("/tmp"),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "share-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "share-test-node".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
            dns_popularity_tracker: None,
            shard_location_index: None,
            consumer_provider_manager: None,
            #[cfg(feature = "caesar")]
            caesar: None,
            #[cfg(feature = "intelligence")]
            engauge_bridge: None,
            #[cfg(feature = "intelligence")]
            federation_manager: None,
            #[cfg(feature = "intelligence")]
            threshold_coordinator: None,

            transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
            release_feed_subscriber: None,
            receipt_validator: Arc::new(
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
        })
    }

    #[tokio::test]
    async fn test_share_send_success() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.send",
            serde_json::json!({
                "asset_id": "test-asset-abc",
                "recipient": "peer-node-xyz"
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "expected success: {:?}", resp.error);
        let result = resp.result.expect("test: result present");
        assert_eq!(result["asset_id"], "test-asset-abc");
        assert_eq!(result["status"], "created");
        assert!(
            result["invite_id"]
                .as_str()
                .expect("test: invite_id")
                .starts_with("inv-"),
        );
    }

    #[tokio::test]
    async fn test_share_send_missing_asset_id() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.send",
            serde_json::json!({"recipient": "peer-node-xyz"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("asset_id"));
    }

    #[tokio::test]
    async fn test_share_send_missing_recipient() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.send",
            serde_json::json!({"asset_id": "test-asset"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("recipient"));
    }

    #[tokio::test]
    async fn test_share_inbox_empty() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("share.inbox", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
        assert!(result["invites"].is_array());
    }

    #[tokio::test]
    async fn test_share_inbox_with_limit() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.inbox",
            serde_json::json!({"limit": 10}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["limit"], 10);
    }

    #[tokio::test]
    async fn test_share_accept_success() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.accept",
            serde_json::json!({"invite_id": "inv-abcdef1234"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["invite_id"], "inv-abcdef1234");
        assert_eq!(result["status"], "accepted");
    }

    #[tokio::test]
    async fn test_share_accept_missing_invite_id() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("share.accept", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("invite_id"));
    }

    #[tokio::test]
    async fn test_share_reject_success() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.reject",
            serde_json::json!({"invite_id": "inv-reject-test"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["invite_id"], "inv-reject-test");
        assert_eq!(result["status"], "rejected");
    }

    #[tokio::test]
    async fn test_share_reject_missing_invite_id() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("share.reject", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("invite_id"));
    }
}
