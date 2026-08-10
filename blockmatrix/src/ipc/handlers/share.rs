// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Share IPC handlers: send, inbox, accept, reject.
//!
//! Exposes P2P file-sharing operations over JSON-RPC 2.0, wired to the
//! `sharing` module (P3, R6 instruction-based retrieval).
//!
//! ## Transmission-payload model (F5)
//!
//! No file bytes are sent. `share.send` builds a [`ShareInvite`] — a recipient
//! bound transmission payload containing the shard map (locate + BLAKE3
//! integrity), the asset's `DecryptionKey` wrapped for the recipient's Kyber
//! public key ([`KeyEnvelope`]), and a FALCON-1024 signature — then delivers
//! it over `TAG_SHARE_INVITE`. Only the recipient can decapsulate the key.
//! `share.accept` unwraps the key with the node's own Kyber secret and writes
//! a secret-free local shard map. `share.inbox`/`share.reject` operate on the
//! shared [`InboxStore`].

use std::path::PathBuf;
use std::sync::Arc;

use hypermesh_lib::NodeEncryptor;

use crate::identity::FalconIdentity;
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use crate::sharing::invite::ShareInvite;
use crate::sharing::key_wrap::KeyEnvelope;
use crate::sharing::shard_map::ShardMap;

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
        let s = state.clone();
        handler.register(
            "share.accept",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_share_accept(params, &s).await })
            }),
        );
    }

    // share.reject -- reject/dismiss a received share invite
    {
        let s = state.clone();
        handler.register(
            "share.reject",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_share_reject(params, &s).await })
            }),
        );
    }
}

fn rpc_err(code: i64, message: impl Into<String>) -> RpcError {
    RpcError {
        code,
        message: message.into(),
        data: None,
    }
}

/// Load the node's dual-key identity (FALCON-1024 + Kyber-1024) from disk.
///
/// The identity lives at `{data_dir}/{node_id}/identity/` — the same path the
/// daemon persists it at boot (see `bin/node/bootstrap.rs`). It holds the Kyber
/// secret used to unwrap the owner's self-custody key envelope and the Kyber
/// public key used to re-wrap for the recipient, plus the FALCON key used to
/// sign the invite. Loading from disk (rather than a `DaemonState` field) keeps
/// the raw secret out of the shared state struct.
fn load_node_identity(state: &DaemonState) -> Result<FalconIdentity, RpcError> {
    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
    FalconIdentity::load_or_create(&identity_dir).map_err(|e| {
        rpc_err(
            INTERNAL_ERROR,
            format!("failed to load node identity for share operation: {e}"),
        )
    })
}

/// Load the owner-local shard map for `asset_id` from `{data_dir}/shard_maps/`.
fn load_owner_shard_map(data_dir: &std::path::Path, asset_id: &str) -> Result<ShardMap, RpcError> {
    let map_path = data_dir.join("shard_maps").join(format!("{asset_id}.json"));
    if !map_path.exists() {
        return Err(rpc_err(
            INVALID_PARAMS,
            format!("no shard map for asset {asset_id}; was it stored on this node?"),
        ));
    }
    let json = std::fs::read_to_string(&map_path)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("read shard map: {e}")))?;
    serde_json::from_str(&json)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("parse shard map: {e}")))
}

/// Decode a recipient Kyber public key supplied as a hex parameter.
fn parse_recipient_kyber_pubkey(params: &serde_json::Value) -> Result<Vec<u8>, RpcError> {
    let hex_key = params
        .get("recipient_kyber_pubkey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            rpc_err(
                INVALID_PARAMS,
                "missing 'recipient_kyber_pubkey' (hex): the recipient's Kyber-1024 \
                 public key is required to wrap the decryption key for them",
            )
        })?;
    hex::decode(hex_key)
        .map_err(|e| rpc_err(INVALID_PARAMS, format!("invalid recipient_kyber_pubkey hex: {e}")))
}

/// Handle `share.send` -- build and deliver a transmission payload to a peer.
///
/// Params:
///   - `asset_id` (string, required): Asset to share (must be stored locally)
///   - `recipient` (string, required): Recipient node ID (or DNS name)
///   - `recipient_kyber_pubkey` (string, required): Recipient's Kyber-1024
///     public key as hex — used to wrap the decryption key so ONLY the
///     recipient can decapsulate it
///   - `asset_name` (string, optional): human-readable name for the invite
async fn handle_share_send(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let asset_id = params
        .get("asset_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| rpc_err(INVALID_PARAMS, "missing or invalid 'asset_id' parameter"))?;

    let recipient = params
        .get("recipient")
        .and_then(|v| v.as_str())
        .ok_or_else(|| rpc_err(INVALID_PARAMS, "missing or invalid 'recipient' parameter"))?;

    // Identity is required to unwrap our own key custody envelope and to sign.
    let identity = load_node_identity(state)?;

    let recipient_kyber_pubkey = parse_recipient_kyber_pubkey(&params)?;

    // Resolve recipient via DNS if it looks like a name rather than a node_id.
    let resolved_recipient = if recipient.len() < 64 {
        match state.dns_resolver.resolve(recipient).await {
            Some(addr) => format!("{recipient} ({addr})"),
            None => recipient.to_string(),
        }
    } else {
        recipient.to_string()
    };

    // 1. Load the owner-local shard map (locate + integrity + self-custody key).
    let map = load_owner_shard_map(&state.data_dir, asset_id)?;

    // 2. Recover the asset's DecryptionKey.
    //    - Encrypted (Private) asset: unwrap our self-custody envelope with our
    //      node Kyber secret, then re-wrap for the recipient.
    //    - Cleartext (Public/Anonymous) asset: no key to custody; the invite
    //      carries an empty envelope and the recipient reconstructs directly.
    let (encrypted_key, key_kem_ciphertext) = match &map.key_envelope {
        Some(env) => {
            let decryption_key = env
                .unwrap_with(identity.kyber_secret_key_bytes())
                .map_err(|e| {
                    rpc_err(
                        INTERNAL_ERROR,
                        format!("failed to unwrap owner key envelope: {e}"),
                    )
                })?;
            let recipient_env = KeyEnvelope::wrap_for(&decryption_key, &recipient_kyber_pubkey)
                .map_err(|e| {
                    rpc_err(INTERNAL_ERROR, format!("failed to wrap key for recipient: {e}"))
                })?;
            (recipient_env.encrypted_key, recipient_env.key_kem_ciphertext)
        }
        None => (Vec::new(), Vec::new()),
    };

    // 3. Assemble the transmission payload. The shard map that travels carries
    //    NO key envelope — only locate + integrity data — so a raw secret key
    //    can never leak into the wire payload.
    let travel_map = ShardMap {
        key_envelope: None,
        ..map.clone()
    };
    let shard_map_json = serde_json::to_vec(&travel_map)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("serialize travel shard map: {e}")))?;

    let asset_name = params
        .get("asset_name")
        .and_then(|v| v.as_str())
        .unwrap_or(asset_id)
        .to_string();

    let invite_id = {
        let mut hasher = blake3::Hasher::new();
        hasher.update(asset_id.as_bytes());
        hasher.update(recipient.as_bytes());
        hasher.update(&state.started_at.elapsed().as_nanos().to_le_bytes());
        format!("inv-{}", &hasher.finalize().to_hex()[..16])
    };

    let created_at = chrono::Utc::now().timestamp();
    let mut invite = ShareInvite::new(
        invite_id.clone(),
        asset_id.to_string(),
        identity.node_id.clone(),
        None,
        recipient.to_string(),
        asset_name,
        map.original_size as u64,
        map.shard_count as u32,
        shard_map_json,
        encrypted_key,
        key_kem_ciphertext,
        created_at,
    );

    // 4. Sign the invite with our FALCON-1024 identity.
    invite
        .sign(&identity)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("failed to sign invite: {e}")))?;

    // 5. The signed transmission payload is now assembled and self-consistent.
    //    Serialize once so it can be framed as `[TAG_SHARE_INVITE] ++ wire`
    //    and delivered to the recipient over STOQ. The travelling payload
    //    never carries a raw secret key (the travel map's envelope is None).
    let invite_wire = serde_json::to_vec(&invite)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("serialize invite: {e}")))?;

    // 6. Deliver over the network to the recipient by node_id. This is a
    //    best-effort peer send: if the recipient is not a currently-connected
    //    peer (no network, dial not possible, stream failed), the invite is
    //    still created and signed — we degrade to `created_undelivered` with
    //    an error field rather than hard-failing the IPC call.
    let (delivered, status, delivery_error) =
        deliver_invite(state, recipient, &invite_wire).await;

    let mut response = serde_json::json!({
        "invite_id": invite_id,
        "asset_id": asset_id,
        "recipient": resolved_recipient,
        "sender": identity.node_id,
        "encrypted": map.key_envelope.is_some(),
        "shard_count": map.shard_count,
        "signed": true,
        "invite_bytes": invite_wire.len(),
        "delivered": delivered,
        "status": status,
    });
    if let Some(err) = delivery_error {
        response["delivery_error"] = serde_json::Value::String(err);
    }
    Ok(response)
}

/// Deliver a signed invite to `recipient` over `TAG_SHARE_INVITE`.
///
/// Frames the wire payload as `[TAG_SHARE_INVITE] ++ invite_wire` and sends it
/// to the recipient node via [`NetworkManager::send_tagged_to_peer`], mirroring
/// the `StoqShardTransport::send_shard` / `broadcast_announcement` pattern. The
/// receiver's `handle_share_invite` (tag 0x05) parses the same framing into the
/// shared `InboxStore`.
///
/// Returns `(delivered, status, delivery_error)`:
///   - `(true, "delivered", None)` on a successful peer send;
///   - `(false, "created_undelivered", Some(reason))` when there is no network
///     manager or the peer send fails (peer not connected / dial failed).
///
/// Delivery failure is never fatal — the invite has already been created and
/// signed, so the caller degrades gracefully.
async fn deliver_invite(
    state: &DaemonState,
    recipient: &str,
    invite_wire: &[u8],
) -> (bool, &'static str, Option<String>) {
    let network = match state.network.as_ref() {
        Some(network) => network,
        None => {
            return (
                false,
                "created_undelivered",
                Some("no network manager: node is running without STOQ transport".into()),
            );
        }
    };

    match network
        .send_tagged_to_peer(
            recipient,
            crate::network::message_handlers::TAG_SHARE_INVITE,
            invite_wire,
        )
        .await
    {
        Ok(()) => (true, "delivered", None),
        Err(e) => (false, "created_undelivered", Some(e.to_string())),
    }
}

/// Handle `share.inbox` -- list pending received share invites.
///
/// Params (all optional):
///   - `limit` (u64): Max invites to return (default 50)
async fn handle_share_inbox(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let inbox = match state.inbox_store.as_ref() {
        Some(inbox) => inbox,
        None => {
            // No inbox configured (e.g. Device-only node without networking).
            return Ok(serde_json::json!({
                "invites": [],
                "count": 0,
                "limit": limit,
            }));
        }
    };

    let invites = inbox.list().await;
    let count = invites.len();
    let items: Vec<serde_json::Value> = invites
        .into_iter()
        .take(limit)
        .map(|inv| {
            serde_json::json!({
                "invite_id": inv.invite_id,
                "asset_id": inv.asset_id,
                "asset_name": inv.asset_name,
                "asset_size": inv.asset_size,
                "shard_count": inv.shard_count,
                "sender_node_id": inv.sender_node_id,
                "sender_name": inv.sender_name,
                "encrypted": !inv.encrypted_key.is_empty(),
                "created_at": inv.created_at,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "invites": items,
        "count": count,
        "limit": limit,
    }))
}

/// Persist a secret-free shard map derived from an accepted invite.
///
/// The map stored on disk carries the (locate + integrity) data from the
/// invite plus — for encrypted assets — a NEW key envelope wrapped for THIS
/// node's own Kyber identity (self-custody). The raw Kyber secret never
/// touches disk; it stays in the node keystore.
fn persist_accepted_map(
    data_dir: &std::path::Path,
    travel_map: &ShardMap,
    self_envelope: Option<KeyEnvelope>,
) -> Result<PathBuf, RpcError> {
    let local_map = ShardMap {
        key_envelope: self_envelope,
        ..travel_map.clone()
    };
    let maps_dir = data_dir.join("shard_maps");
    std::fs::create_dir_all(&maps_dir)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("create shard_maps dir: {e}")))?;
    let map_path = maps_dir.join(format!("{}.json", local_map.asset_id));
    let json = serde_json::to_string_pretty(&local_map)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("serialize accepted map: {e}")))?;
    std::fs::write(&map_path, json.as_bytes())
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("write accepted map: {e}")))?;
    Ok(map_path)
}

/// Handle `share.accept` -- accept a received transmission payload.
///
/// Params:
///   - `invite_id` (string, required): ID of the invite to accept
///
/// Flow: look up the invite, unwrap its key envelope with our own Kyber
/// secret, re-wrap for self-custody, write a secret-free local shard map, and
/// remove the invite from the inbox. The recipient can then `fetch` the asset.
async fn handle_share_accept(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let invite_id = params
        .get("invite_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| rpc_err(INVALID_PARAMS, "missing or invalid 'invite_id' parameter"))?;

    let inbox = state.inbox_store.as_ref().ok_or_else(|| {
        rpc_err(INTERNAL_ERROR, "sharing inbox not configured on this node")
    })?;
    let identity = load_node_identity(state)?;

    let invite = inbox
        .get(invite_id)
        .await
        .ok_or_else(|| rpc_err(INVALID_PARAMS, format!("invite {invite_id} not found")))?;

    // Deserialize the transmission payload's travelling shard map.
    let travel_map: ShardMap = serde_json::from_slice(&invite.shard_map_json)
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("parse invite shard map: {e}")))?;

    // Recover the decryption key (if any) and re-wrap for self-custody.
    let self_envelope = if invite.encrypted_key.is_empty() {
        // Cleartext asset: no key to custody.
        None
    } else {
        let recipient_env = KeyEnvelope {
            encrypted_key: invite.encrypted_key.clone(),
            key_kem_ciphertext: invite.key_kem_ciphertext.clone(),
        };
        let decryption_key = recipient_env
            .unwrap_with(identity.kyber_secret_key_bytes())
            .map_err(|e| {
                rpc_err(
                    INTERNAL_ERROR,
                    format!("failed to unwrap key (are you the intended recipient?): {e}"),
                )
            })?;
        // Re-wrap for our own node Kyber pubkey so self-fetch never needs the
        // raw secret to be persisted.
        let self_env = KeyEnvelope::wrap_for(&decryption_key, identity.encryption_public_key())
            .map_err(|e| rpc_err(INTERNAL_ERROR, format!("failed to self-wrap key: {e}")))?;
        Some(self_env)
    };

    let encrypted = self_envelope.is_some();
    let map_path = persist_accepted_map(&state.data_dir, &travel_map, self_envelope)?;

    // Remove the invite now that it has been materialized as a local map.
    inbox.remove(invite_id).await;

    Ok(serde_json::json!({
        "invite_id": invite_id,
        "asset_id": travel_map.asset_id,
        "shard_count": travel_map.shard_count,
        "encrypted": encrypted,
        "shard_map": map_path.display().to_string(),
        "status": "accepted",
    }))
}

/// Handle `share.reject` -- reject/dismiss a received share invite.
///
/// Params:
///   - `invite_id` (string, required): ID of the invite to reject
async fn handle_share_reject(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let invite_id = params
        .get("invite_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| rpc_err(INVALID_PARAMS, "missing or invalid 'invite_id' parameter"))?;

    let removed = match state.inbox_store.as_ref() {
        Some(inbox) => inbox.remove(invite_id).await.is_some(),
        None => false,
    };

    Ok(serde_json::json!({
        "invite_id": invite_id,
        "removed": removed,
        "status": "rejected",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pipeline::orchestrator::DecryptionKey;
    use crate::assets::pipeline::ShardMetadata;
    use crate::blockchain::node_chain::NodeBlockchain;
    use crate::bootstrap::DnsResolver;
    use crate::ipc::handler::RequestHandler;
    use crate::ipc::protocol::RpcRequest;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::network::shard_store::ShardStore;
    use crate::persistence::{PersistenceConfig, PersistenceManager};
    use crate::sharing::inbox::InboxStore;
    use std::time::Instant;
    use trustchain::FalconIdentity;

    /// The node_id used by the test DaemonState. The share handlers derive the
    /// on-disk identity path from `data_dir.join(node_id).join("identity")`, so
    /// tests that need an identity must persist it there via [`disk_identity`].
    const TEST_NODE_ID: &str = "share-test-node";

    /// Create (and persist) the node's dual-key identity at the exact path the
    /// share handlers load it from: `{data_dir}/{TEST_NODE_ID}/identity`.
    ///
    /// Returns the loaded identity so the test can use its public Kyber key to
    /// wrap owner maps / recipient envelopes. The raw secret stays on disk and
    /// is never held in `DaemonState`.
    fn disk_identity(data_dir: &std::path::Path) -> FalconIdentity {
        let identity_dir = data_dir.join(TEST_NODE_ID).join("identity");
        FalconIdentity::load_or_create(&identity_dir).expect("test: disk identity")
    }

    /// Build a DaemonState with an optional inbox, rooted at `data_dir`.
    async fn test_state_at(
        data_dir: PathBuf,
        inbox: Option<Arc<InboxStore>>,
    ) -> Arc<DaemonState> {
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
            node_id: TEST_NODE_ID.into(),
            network_id: "trustnet-test".into(),
            data_dir,
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
            shard_location_index: None,
            consumer_provider_manager: None,
            #[cfg(feature = "caesar")]
            caesar: None,
            #[cfg(feature = "intelligence")]
            ngauge_bridge: None,
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
            capability_token_issuer: None,
            revocation_registry: Arc::new(crate::auth::RevocationRegistry::new()),
            light_sync_manager: None,
            catalog_registry: None,
            inbox_store: inbox,
        })
    }

    async fn bare_state() -> Arc<DaemonState> {
        test_state_at(PathBuf::from("/tmp"), None).await
    }

    fn write_owner_map(
        data_dir: &std::path::Path,
        asset_id: &str,
        owner: &FalconIdentity,
        encrypted: bool,
    ) {
        let key_envelope = if encrypted {
            let dk = DecryptionKey::Kyber {
                ciphertext_kem: vec![0x01; 128],
                nonce: vec![0x02; 12],
                original_size: 100,
                secret_key: vec![0xEE; 3168],
            };
            Some(
                KeyEnvelope::wrap_for(&dk, &owner.kyber_public_key)
                    .expect("test: wrap for owner"),
            )
        } else {
            None
        };
        let map = ShardMap {
            asset_id: asset_id.into(),
            shard_hashes: vec!["0".repeat(64), "1".repeat(64)],
            key_envelope,
            shard_count: 2,
            original_size: 100,
            shard_metadata: vec![
                ShardMetadata {
                    index: 0,
                    is_parity: false,
                    size: 50,
                    original_size: 50,
                    hash: "0".repeat(64),
                },
                ShardMetadata {
                    index: 1,
                    is_parity: false,
                    size: 50,
                    original_size: 50,
                    hash: "1".repeat(64),
                },
            ],
        };
        let maps_dir = data_dir.join("shard_maps");
        std::fs::create_dir_all(&maps_dir).expect("test: mkdir maps");
        std::fs::write(
            maps_dir.join(format!("{asset_id}.json")),
            serde_json::to_string_pretty(&map).expect("test: ser map"),
        )
        .expect("test: write map");
    }

    #[tokio::test]
    async fn test_share_send_missing_asset_id() {
        let state = bare_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.send",
            serde_json::json!({"recipient": "peer-node-xyz"}),
        );
        let resp = handler.dispatch(req).await;
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("asset_id"));
    }

    #[tokio::test]
    async fn test_share_send_missing_recipient() {
        let state = bare_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("share.send", serde_json::json!({"asset_id": "a"}));
        let resp = handler.dispatch(req).await;
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("recipient"));
    }

    #[tokio::test]
    async fn test_share_send_requires_recipient_pubkey() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let owner = disk_identity(dir.path());
        write_owner_map(dir.path(), "asset-a", &owner, true);
        let state = test_state_at(dir.path().to_path_buf(), None).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "share.send",
            serde_json::json!({"asset_id": "asset-a", "recipient": "peer"}),
        );
        let resp = handler.dispatch(req).await;
        let err = resp.error.expect("test: error");
        assert!(err.message.contains("recipient_kyber_pubkey"));
    }

    /// Full round-trip: sender wraps for recipient, recipient accepts and
    /// materializes a secret-free local map, and the recovered key matches.
    #[tokio::test]
    async fn test_share_send_accept_roundtrip() {
        // Sender node — identity persisted at the handler's on-disk path.
        let send_dir = tempfile::tempdir().expect("test: send tempdir");
        let sender = disk_identity(send_dir.path());
        let asset_id = "roundtrip-asset";
        write_owner_map(send_dir.path(), asset_id, &sender, true);
        let sender_state = test_state_at(send_dir.path().to_path_buf(), None).await;
        let mut sender_handler = RequestHandler::new();
        register(&mut sender_handler, &sender_state);

        // Recipient node — its own identity persisted at its own path.
        let recv_dir = tempfile::tempdir().expect("test: recv tempdir");
        let recipient = disk_identity(recv_dir.path());
        let recv_inbox = Arc::new(InboxStore::new(Some(recv_dir.path().join("inbox"))));
        let recipient_state =
            test_state_at(recv_dir.path().to_path_buf(), Some(recv_inbox.clone())).await;
        let mut recipient_handler = RequestHandler::new();
        register(&mut recipient_handler, &recipient_state);

        // Sender builds the invite (delivery is None → created_undelivered).
        let send_req = RpcRequest::new(
            "share.send",
            serde_json::json!({
                "asset_id": asset_id,
                "recipient": recipient.node_id,
                "recipient_kyber_pubkey": hex::encode(&recipient.kyber_public_key),
                "asset_name": "photo.jpg",
            }),
        );
        let send_resp = sender_handler.dispatch(send_req).await;
        assert!(send_resp.error.is_none(), "send failed: {:?}", send_resp.error);
        let send_result = send_resp.result.expect("test: send result");
        assert_eq!(send_result["encrypted"], true);
        assert_eq!(send_result["signed"], true);
        let invite_id = send_result["invite_id"]
            .as_str()
            .expect("test: invite_id")
            .to_string();

        // Reconstruct the invite the sender would have transmitted and inject
        // it into the recipient's inbox (simulating the TAG_SHARE_INVITE wire).
        let (encrypted_key, kem_ct) = {
            let map = load_owner_shard_map(send_dir.path(), asset_id)
                .expect("test: load owner map");
            let dk = map
                .key_envelope
                .as_ref()
                .expect("test: envelope")
                .unwrap_with(sender.kyber_secret_key_bytes())
                .expect("test: unwrap owner");
            let env = KeyEnvelope::wrap_for(&dk, &recipient.kyber_public_key)
                .expect("test: wrap recipient");
            (env.encrypted_key, env.key_kem_ciphertext)
        };
        let travel_map = {
            let map = load_owner_shard_map(send_dir.path(), asset_id)
                .expect("test: reload");
            ShardMap { key_envelope: None, ..map }
        };
        let shard_map_json = serde_json::to_vec(&travel_map).expect("test: ser travel");
        let mut invite = ShareInvite::new(
            invite_id.clone(),
            asset_id.into(),
            sender.node_id.clone(),
            None,
            recipient.node_id.clone(),
            "photo.jpg".into(),
            travel_map.original_size as u64,
            travel_map.shard_count as u32,
            shard_map_json,
            encrypted_key,
            kem_ct,
            chrono::Utc::now().timestamp(),
        );
        invite.sign(&sender).expect("test: sign invite");

        // The travelling shard map must contain NO raw secret bytes.
        let raw_secret_window = vec![0xEEu8; 64];
        assert!(
            !invite
                .shard_map_json
                .windows(raw_secret_window.len())
                .any(|w| w == raw_secret_window.as_slice()),
            "raw secret leaked into transmitted shard map"
        );

        recv_inbox.add(invite).await.expect("test: inbox add");

        // Recipient lists inbox and sees the invite.
        let inbox_resp = recipient_handler
            .dispatch(RpcRequest::new("share.inbox", serde_json::json!({})))
            .await;
        assert_eq!(inbox_resp.result.expect("test: inbox")["count"], 1);

        // Recipient accepts.
        let accept_resp = recipient_handler
            .dispatch(RpcRequest::new(
                "share.accept",
                serde_json::json!({"invite_id": invite_id}),
            ))
            .await;
        assert!(
            accept_resp.error.is_none(),
            "accept failed: {:?}",
            accept_resp.error
        );
        let accept_result = accept_resp.result.expect("test: accept result");
        assert_eq!(accept_result["status"], "accepted");
        assert_eq!(accept_result["asset_id"], asset_id);
        assert_eq!(accept_result["encrypted"], true);

        // The recipient's local map is secret-free and self-custodied.
        let local_map = load_owner_shard_map(recv_dir.path(), asset_id)
            .expect("test: load recipient map");
        assert!(local_map.key_envelope.is_some());
        let local_json = std::fs::read(
            recv_dir.path().join("shard_maps").join(format!("{asset_id}.json")),
        )
        .expect("test: read local map");
        assert!(
            !local_json
                .windows(raw_secret_window.len())
                .any(|w| w == raw_secret_window.as_slice()),
            "raw secret leaked into recipient's local map"
        );

        // Recipient can recover the exact original decryption key.
        let recovered = local_map
            .key_envelope
            .expect("test: local envelope")
            .unwrap_with(recipient.kyber_secret_key_bytes())
            .expect("test: recipient unwrap");
        assert!(matches!(recovered, DecryptionKey::Kyber { .. }));

        // Invite consumed from inbox.
        assert_eq!(recv_inbox.count().await, 0);
    }

    /// Wire-delivery loopback: `share.send` builds + signs an invite, we frame
    /// it exactly as the network send path does (`[TAG_SHARE_INVITE] ++ wire`),
    /// feed those bytes straight into the receiver's parse/store core
    /// (`parse_and_store_share_invite`), and confirm the recipient's inbox gets
    /// the invite AND that `share.accept` then round-trips.
    ///
    /// This proves the framing `share.send` produces is exactly what the
    /// receiver dispatch accepts, without needing a full two-node STOQ harness.
    /// The `delivered:false, created_undelivered` degradation (network=None in
    /// test state) is asserted here too.
    #[tokio::test]
    async fn test_share_send_wire_framing_loopback() {
        use crate::network::message_handlers::{parse_and_store_share_invite, TAG_SHARE_INVITE};

        // Sender node with an identity + owner map at the handler's on-disk path.
        let send_dir = tempfile::tempdir().expect("test: send tempdir");
        let sender = disk_identity(send_dir.path());
        let asset_id = "loopback-asset";
        write_owner_map(send_dir.path(), asset_id, &sender, true);
        let sender_state = test_state_at(send_dir.path().to_path_buf(), None).await;
        let mut sender_handler = RequestHandler::new();
        register(&mut sender_handler, &sender_state);

        // Recipient node with its own identity + shared inbox.
        let recv_dir = tempfile::tempdir().expect("test: recv tempdir");
        let recipient = disk_identity(recv_dir.path());
        let recv_inbox = Arc::new(InboxStore::new(Some(recv_dir.path().join("inbox"))));
        let recipient_state =
            test_state_at(recv_dir.path().to_path_buf(), Some(recv_inbox.clone())).await;
        let mut recipient_handler = RequestHandler::new();
        register(&mut recipient_handler, &recipient_state);

        // share.send builds + signs the invite. With network=None the wire
        // send degrades gracefully to created_undelivered (asserted below),
        // but the signed invite_wire it produces is what a connected peer
        // would receive — so we reconstruct that exact framing here.
        let send_req = RpcRequest::new(
            "share.send",
            serde_json::json!({
                "asset_id": asset_id,
                "recipient": recipient.node_id,
                "recipient_kyber_pubkey": hex::encode(&recipient.kyber_public_key),
                "asset_name": "clip.mp4",
            }),
        );
        let send_resp = sender_handler.dispatch(send_req).await;
        assert!(send_resp.error.is_none(), "send failed: {:?}", send_resp.error);
        let send_result = send_resp.result.expect("test: send result");
        // Graceful degradation: no network manager in test state.
        assert_eq!(send_result["delivered"], false);
        assert_eq!(send_result["status"], "created_undelivered");
        assert!(send_result["delivery_error"].is_string());
        let invite_id = send_result["invite_id"]
            .as_str()
            .expect("test: invite_id")
            .to_string();

        // Rebuild the exact signed `invite_wire` share.send serialized, then
        // frame it as the network send path does: [TAG_SHARE_INVITE] ++ wire.
        let invite_wire = {
            let map = load_owner_shard_map(send_dir.path(), asset_id).expect("test: owner map");
            let dk = map
                .key_envelope
                .as_ref()
                .expect("test: envelope")
                .unwrap_with(sender.kyber_secret_key_bytes())
                .expect("test: unwrap owner");
            let recipient_env = KeyEnvelope::wrap_for(&dk, &recipient.kyber_public_key)
                .expect("test: wrap recipient");
            let travel_map = ShardMap { key_envelope: None, ..map.clone() };
            let shard_map_json = serde_json::to_vec(&travel_map).expect("test: ser travel");
            let mut invite = ShareInvite::new(
                invite_id.clone(),
                asset_id.into(),
                sender.node_id.clone(),
                None,
                recipient.node_id.clone(),
                "clip.mp4".into(),
                travel_map.original_size as u64,
                travel_map.shard_count as u32,
                shard_map_json,
                recipient_env.encrypted_key,
                recipient_env.key_kem_ciphertext,
                chrono::Utc::now().timestamp(),
            );
            invite.sign(&sender).expect("test: sign invite");
            serde_json::to_vec(&invite).expect("test: ser invite")
        };
        let mut framed = Vec::with_capacity(1 + invite_wire.len());
        framed.push(TAG_SHARE_INVITE);
        framed.extend_from_slice(&invite_wire);

        // Feed the framed bytes straight into the receiver's parse/store core —
        // exactly what `handle_share_invite` does on TAG_SHARE_INVITE dispatch.
        let stored_id = parse_and_store_share_invite(&framed, &recv_inbox)
            .await
            .expect("test: receiver accepts share.send framing");
        assert_eq!(stored_id, invite_id);
        assert_eq!(recv_inbox.count().await, 1);

        // The recipient sees it in the inbox and can accept it — full round-trip.
        let inbox_resp = recipient_handler
            .dispatch(RpcRequest::new("share.inbox", serde_json::json!({})))
            .await;
        assert_eq!(inbox_resp.result.expect("test: inbox")["count"], 1);

        let accept_resp = recipient_handler
            .dispatch(RpcRequest::new(
                "share.accept",
                serde_json::json!({"invite_id": invite_id}),
            ))
            .await;
        assert!(
            accept_resp.error.is_none(),
            "accept failed: {:?}",
            accept_resp.error
        );
        let accept_result = accept_resp.result.expect("test: accept result");
        assert_eq!(accept_result["status"], "accepted");
        assert_eq!(accept_result["asset_id"], asset_id);
        assert_eq!(accept_result["encrypted"], true);

        // Recipient recovers the exact decryption key from its self-custodied map.
        let local_map =
            load_owner_shard_map(recv_dir.path(), asset_id).expect("test: recipient map");
        let recovered = local_map
            .key_envelope
            .expect("test: local envelope")
            .unwrap_with(recipient.kyber_secret_key_bytes())
            .expect("test: recipient unwrap");
        assert!(matches!(recovered, DecryptionKey::Kyber { .. }));
        assert_eq!(recv_inbox.count().await, 0);
    }

    #[tokio::test]
    async fn test_share_inbox_empty_without_store() {
        let state = bare_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let resp = handler
            .dispatch(RpcRequest::new("share.inbox", serde_json::json!({})))
            .await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
        assert!(result["invites"].is_array());
    }

    #[tokio::test]
    async fn test_share_accept_missing_invite_id() {
        let state = bare_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let resp = handler
            .dispatch(RpcRequest::new("share.accept", serde_json::json!({})))
            .await;
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("invite_id"));
    }

    #[tokio::test]
    async fn test_share_reject_removes_invite() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let inbox = Arc::new(InboxStore::new(Some(dir.path().join("inbox"))));
        // Seed an invite (reject needs no key custody or identity).
        let invite = ShareInvite::new(
            "inv-reject".into(),
            "asset".into(),
            "sender".into(),
            None,
            TEST_NODE_ID.into(),
            "f.bin".into(),
            1,
            1,
            b"{}".to_vec(),
            Vec::new(),
            Vec::new(),
            0,
        );
        inbox.add(invite).await.expect("test: add");
        let state = test_state_at(dir.path().to_path_buf(), Some(inbox.clone())).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let resp = handler
            .dispatch(RpcRequest::new(
                "share.reject",
                serde_json::json!({"invite_id": "inv-reject"}),
            ))
            .await;
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "rejected");
        assert_eq!(result["removed"], true);
        assert_eq!(inbox.count().await, 0);
    }

    #[tokio::test]
    async fn test_share_reject_missing_invite_id() {
        let state = bare_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let resp = handler
            .dispatch(RpcRequest::new("share.reject", serde_json::json!({})))
            .await;
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("invite_id"));
    }
}
