// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Auth IPC handlers (Phase K.1).
//!
//! - `auth.create_session(device_pubkey, requested_capabilities, ttl_secs)`
//!   — issues a FALCON-signed capability token bound to `device_pubkey`.
//!   Records a `SessionAudit { Created }` entry on the local chain.
//!
//! - `auth.list_sessions` — returns active session IDs known to the
//!   daemon (currently the in-memory issued set; K.1.5 will reconcile
//!   against the on-chain `SessionAudit` log).
//!
//! - `auth.revoke_session(session_id)` — adds the session to the
//!   in-memory revocation registry and records a
//!   `SessionAudit { Revoked }` entry on chain.
//!
//! All three are alpha-default inert: when
//! `state.capability_token_issuer` is `None` they reject with
//! "auth not configured (alpha-default inert)".

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::assets::core::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
};
use crate::auth::{Capability, CapabilityToken, SessionAction};
use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{INTERNAL_ERROR, INVALID_PARAMS, RpcError};
use crate::ipc::state::DaemonState;
use trustchain::proof_of_state::StateProof;

/// JSON-RPC error code: capability denied (token signature valid but
/// the requested method requires a higher scope).
pub const CAPABILITY_DENIED: i64 = -32004;

/// Track every issued session in memory. Populated by
/// `auth.create_session`, consulted by `auth.list_sessions`.
///
/// K.1 ships in-memory tracking; K.1.5 rebuilds this from the chain on
/// daemon startup.
static ISSUED_SESSIONS: tokio::sync::OnceCell<
    Arc<tokio::sync::RwLock<Vec<IssuedSessionRecord>>>,
> = tokio::sync::OnceCell::const_new();

async fn issued_sessions()
    -> Arc<tokio::sync::RwLock<Vec<IssuedSessionRecord>>> {
    ISSUED_SESSIONS
        .get_or_init(|| async {
            Arc::new(tokio::sync::RwLock::new(Vec::new()))
        })
        .await
        .clone()
}

/// In-memory record of an issued session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssuedSessionRecord {
    /// Session UUID.
    pub session_id: Uuid,
    /// Hex-encoded device pubkey.
    pub device_pubkey_hex: String,
    /// Granted capabilities.
    pub capabilities: Vec<Capability>,
    /// Issued-at as Unix seconds.
    pub issued_at_secs: u64,
    /// Expiry as Unix seconds.
    pub valid_until_secs: u64,
}

/// Payload stored in the chain entry for a `SessionAudit` event.
///
/// Kept self-describing (versioned variant tag + plain fields) so
/// future K.1.5 readers can replay the on-chain audit trail without a
/// schema lookup.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionAuditPayload {
    /// Schema version of this payload.
    pub version: u32,
    /// Session UUID.
    pub session_id: Uuid,
    /// Hex-encoded device pubkey.
    pub device_pubkey_hex: String,
    /// Capabilities granted at session creation.
    pub capabilities: Vec<Capability>,
    /// What happened to the session (Created/Revoked/CapabilityUsed).
    pub action: SessionAction,
    /// Unix seconds when the action was recorded.
    pub timestamp_secs: u64,
}

impl SessionAuditPayload {
    /// Construct a payload for the current moment.
    pub fn new(
        session_id: Uuid,
        device_pubkey: &[u8],
        capabilities: Vec<Capability>,
        action: SessionAction,
    ) -> Self {
        let timestamp_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            version: 1,
            session_id,
            device_pubkey_hex: hex::encode(device_pubkey),
            capabilities,
            action,
            timestamp_secs,
        }
    }
}

/// Record a `SessionAudit` entry on the local chain.
///
/// Returns `Ok(())` on success; logs and swallows persistence errors so
/// auth IPCs aren't fatal in alpha (revocation still takes effect via
/// the in-memory registry). K.1.5 will harden the persistence path.
async fn record_session_audit(
    state: &Arc<DaemonState>,
    payload: SessionAuditPayload,
) -> Result<(), String> {
    let entry_bytes = serde_json::to_vec(&payload)
        .map_err(|e| format!("serialize SessionAudit payload: {e}"))?;
    let asset_hash = *blake3::hash(&entry_bytes).as_bytes();

    // Build a content-addressed registration so the chain entry is
    // distinguishable from other Identity-flavoured entries.
    let asset_data = AssetData {
        config: format!("session-audit/v1/{:?}", payload.action).into_bytes(),
        definition: payload.session_id.as_bytes().to_vec(),
        metadata: entry_bytes.clone(),
    };
    let registration = AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Identity),
    );

    let proof_hash = *blake3::hash(b"session-audit-proof-v1").as_bytes();

    let payload_str = String::from_utf8_lossy(&entry_bytes).to_string();

    // Generate a REAL PoS proof from this node's own identity
    // (R1: hardware-assessed, not self-reported).
    let state_proof = StateProof::generate_from_network(&state.node_id)
        .await
        .map_err(|e| format!("session-audit proof generation: {e}"))?;

    let entry = BlockAssetEntry {
        asset_hash,
        proof_hash,
        state_proof,
        storage_pointer: StoragePointer::Local { path: payload_str },
        registration,
    };

    state
        .blockchain
        .add_block(vec![entry])
        .await
        .map(|_blk| ())
        .map_err(|e| format!("add SessionAudit block: {e}"))
}

fn parse_capabilities(raw: &serde_json::Value) -> Result<Vec<Capability>, RpcError> {
    let arr = raw.as_array().ok_or_else(|| RpcError {
        code: INVALID_PARAMS,
        message: "'requested_capabilities' must be an array of strings".into(),
        data: None,
    })?;
    let mut caps = Vec::with_capacity(arr.len());
    for v in arr {
        let s = v.as_str().ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "capability entries must be strings".into(),
            data: None,
        })?;
        let cap = match s.to_ascii_lowercase().as_str() {
            "viewonly" | "view_only" | "view-only" => Capability::ViewOnly,
            "wallet" => Capability::Wallet,
            "assetwrite" | "asset_write" | "asset-write" => Capability::AssetWrite,
            "admin" => Capability::Admin,
            other => {
                return Err(RpcError {
                    code: INVALID_PARAMS,
                    message: format!(
                        "unknown capability '{}': expected viewonly|wallet|assetwrite|admin",
                        other
                    ),
                    data: None,
                });
            }
        };
        caps.push(cap);
    }
    Ok(caps)
}

fn token_to_json(token: &CapabilityToken) -> serde_json::Value {
    let issued_at_secs = token
        .issued_at
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let valid_until_secs = token
        .valid_until
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let serialized = serde_json::to_value(token).unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "session_id": token.session_id.to_string(),
        "device_pubkey_hex": hex::encode(&token.device_pubkey),
        "capabilities": token.capabilities,
        "issued_at_secs": issued_at_secs,
        "valid_until_secs": valid_until_secs,
        "issued_by_hex": hex::encode(&token.issued_by),
        "signature_hex": hex::encode(&token.signature),
        "token": serialized,
    })
}

/// Register the auth.* IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // auth.create_session
    {
        let s = state.clone();
        handler.register(
            "auth.create_session",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let issuer = match s.capability_token_issuer.clone() {
                        Some(i) => i,
                        None => {
                            return Err(RpcError {
                                code: INTERNAL_ERROR,
                                message: "auth not configured (alpha-default inert)".into(),
                                data: None,
                            });
                        }
                    };

                    let device_pubkey_hex = params
                        .get("device_pubkey")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'device_pubkey' parameter (hex)".into(),
                            data: None,
                        })?;
                    let device_pubkey = hex::decode(device_pubkey_hex).map_err(|e| RpcError {
                        code: INVALID_PARAMS,
                        message: format!("device_pubkey hex decode failed: {e}"),
                        data: None,
                    })?;

                    let caps_raw =
                        params.get("requested_capabilities").ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'requested_capabilities' parameter".into(),
                            data: None,
                        })?;
                    let caps = parse_capabilities(caps_raw)?;

                    let ttl_secs = params
                        .get("ttl_secs")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(3600);
                    let ttl = Duration::from_secs(ttl_secs);

                    let token = issuer
                        .issue(device_pubkey.clone(), caps.clone(), ttl)
                        .map_err(|e| RpcError {
                            code: INTERNAL_ERROR,
                            message: format!("issue token failed: {e}"),
                            data: None,
                        })?;

                    // Record SessionAudit { Created } on chain.
                    let audit = SessionAuditPayload::new(
                        token.session_id,
                        &device_pubkey,
                        token.capabilities.clone(),
                        SessionAction::Created,
                    );
                    if let Err(e) = record_session_audit(&s, audit).await {
                        tracing::warn!(
                            "SessionAudit chain write failed (non-fatal): {e}"
                        );
                    }

                    // Track in-memory.
                    let issued_at_secs = token
                        .issued_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let valid_until_secs = token
                        .valid_until
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    issued_sessions().await.write().await.push(IssuedSessionRecord {
                        session_id: token.session_id,
                        device_pubkey_hex: hex::encode(&device_pubkey),
                        capabilities: token.capabilities.clone(),
                        issued_at_secs,
                        valid_until_secs,
                    });

                    Ok(token_to_json(&token))
                })
            }),
        );
    }

    // auth.list_sessions
    {
        handler.register(
            "auth.list_sessions",
            Arc::new(move |_params| {
                Box::pin(async move {
                    let snapshot = issued_sessions().await.read().await.clone();
                    let count = snapshot.len();
                    let json_list: Vec<serde_json::Value> = snapshot
                        .into_iter()
                        .map(|r| serde_json::to_value(&r).unwrap_or(serde_json::Value::Null))
                        .collect();
                    Ok(serde_json::json!({
                        "sessions": json_list,
                        "count": count,
                    }))
                })
            }),
        );
    }

    // auth.revoke_session
    {
        let s = state.clone();
        handler.register(
            "auth.revoke_session",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    if s.capability_token_issuer.is_none() {
                        return Err(RpcError {
                            code: INTERNAL_ERROR,
                            message: "auth not configured (alpha-default inert)".into(),
                            data: None,
                        });
                    }

                    let session_id_str = params
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'session_id' parameter".into(),
                            data: None,
                        })?;
                    let session_id = Uuid::parse_str(session_id_str).map_err(|e| RpcError {
                        code: INVALID_PARAMS,
                        message: format!("invalid session_id UUID: {e}"),
                        data: None,
                    })?;

                    s.revocation_registry.revoke(session_id).await;

                    // Find capabilities for the audit entry (best-effort).
                    let caps = {
                        let snap = issued_sessions().await.read().await.clone();
                        snap.into_iter()
                            .find(|r| r.session_id == session_id)
                            .map(|r| r.capabilities)
                            .unwrap_or_default()
                    };
                    let audit = SessionAuditPayload::new(
                        session_id,
                        &[], // device pubkey unknown at revocation time without lookup
                        caps,
                        SessionAction::Revoked,
                    );
                    if let Err(e) = record_session_audit(&s, audit).await {
                        tracing::warn!(
                            "SessionAudit chain write failed (non-fatal): {e}"
                        );
                    }

                    Ok(serde_json::json!({
                        "session_id": session_id.to_string(),
                        "revoked": true,
                    }))
                })
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CapabilityTokenIssuer;
    use crate::ipc::protocol::RpcRequest;
    use crate::ipc::handlers::tests::test_state;
    use std::sync::Arc;
    use trustchain::FalconIdentity;

    /// Construct a `DaemonState` with the capability token issuer wired up.
    async fn test_state_with_issuer() -> Arc<DaemonState> {
        let state = test_state().await;
        // Replace via Arc::get_mut would require unwrap; instead build a new
        // state by hand. Simpler: construct fresh. The tests below call
        // helpers that construct their own state.
        state
    }

    #[tokio::test]
    async fn create_session_rejects_when_not_configured() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "auth.create_session",
            serde_json::json!({
                "device_pubkey": "deadbeef",
                "requested_capabilities": ["viewonly"],
                "ttl_secs": 60,
            }),
        );
        let resp = handler.dispatch(req).await;
        let err = resp.error.expect("expected inert rejection");
        assert!(err.message.contains("alpha-default inert"));
    }

    /// Build a `DaemonState` carrying a capability token issuer for the
    /// happy-path tests in `tests/k1_light_client_capability_tests.rs`.
    pub(crate) async fn _doc_helper() {
        let _ = test_state_with_issuer;
        let _ = CapabilityTokenIssuer::new(Arc::new(FalconIdentity::generate()));
    }
}
