// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! TrustChain IPC handlers: CA status, certificates, identity, federation.
//!
//! Alpha: The node uses self-signed certificates for bootstrap (Phase 0). These
//! handlers return the identity key info from disk and structured status for the
//! CA, certificate list, and federation peers.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use crate::ipc::state::DaemonState;

fn invalid_params(msg: impl Into<String>) -> RpcError {
    RpcError {
        code: INVALID_PARAMS,
        message: msg.into(),
        data: None,
    }
}

fn internal_error(msg: impl Into<String>) -> RpcError {
    RpcError {
        code: INTERNAL_ERROR,
        message: msg.into(),
        data: None,
    }
}

/// Register TrustChain-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // trustchain.request_cert -- request certificate signing
    // (local CA in Phase 0, threshold-mode coordinator when wired)
    {
        let s = state.clone();
        handler.register(
            "trustchain.request_cert",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_request_cert(&s, params).await })
            }),
        );
    }

    // trustchain.status -- CA status
    {
        let s = state.clone();
        handler.register(
            "trustchain.status",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_status(&s) })
            }),
        );
    }

    // trustchain.certs -- issued certificates
    {
        let s = state.clone();
        handler.register(
            "trustchain.certs",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_certs(&s) })
            }),
        );
    }

    // trustchain.identity -- FALCON + Kyber key summary
    {
        let s = state.clone();
        handler.register(
            "trustchain.identity",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_identity(&s) })
            }),
        );
    }

    // trustchain.federation -- federation peers
    {
        let s = state.clone();
        handler.register(
            "trustchain.federation",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_federation(&s).await })
            }),
        );
    }
}

/// CA status: bootstrap phase, cert type, key algorithm info.
fn handle_status(state: &DaemonState) -> Result<serde_json::Value, RpcError> {
    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
    let has_falcon = identity_dir.join("falcon_pubkey.der").exists();
    let has_kyber = identity_dir.join("kyber_pubkey.der").exists();

    let phase = if has_falcon && has_kyber {
        "bootstrap_with_keys"
    } else {
        "bootstrap_self_signed"
    };

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "ca_phase": phase,
        "cert_type": "self_signed",
        "signing_algorithm": "FALCON-1024",
        "encryption_algorithm": "Kyber-1024",
        "key_exchange": "X25519MLKEM768",
        "has_falcon_key": has_falcon,
        "has_kyber_key": has_kyber,
        "distributed_ca": false,
        "status": "alpha",
    }))
}

/// List issued certificates. Alpha: only the node's own self-signed cert.
fn handle_certs(state: &DaemonState) -> Result<serde_json::Value, RpcError> {
    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
    let cert_path = identity_dir.join("node_cert.der");
    let has_cert = cert_path.exists();

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "certificates": if has_cert {
            serde_json::json!([{
                "subject": state.node_id,
                "issuer": "self",
                "type": "self_signed",
                "algorithm": "FALCON-1024",
                "path": cert_path.to_string_lossy(),
            }])
        } else {
            serde_json::json!([])
        },
        "count": if has_cert { 1 } else { 0 },
        "status": "alpha",
    }))
}

/// Identity key summary: public key sizes, fingerprints (hex-encoded).
fn handle_identity(state: &DaemonState) -> Result<serde_json::Value, RpcError> {
    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
    let falcon_pk_path = identity_dir.join("falcon_pubkey.der");
    let kyber_pk_path = identity_dir.join("kyber_pubkey.der");

    let falcon_info = match std::fs::read(&falcon_pk_path) {
        Ok(pk) => serde_json::json!({
            "present": true,
            "bytes": pk.len(),
            "fingerprint": hex::encode(&pk[..32.min(pk.len())]),
        }),
        Err(_) => serde_json::json!({
            "present": false,
            "bytes": 0,
            "fingerprint": null,
        }),
    };

    let kyber_info = match std::fs::read(&kyber_pk_path) {
        Ok(pk) => serde_json::json!({
            "present": true,
            "bytes": pk.len(),
            "fingerprint": hex::encode(&pk[..32.min(pk.len())]),
        }),
        Err(_) => serde_json::json!({
            "present": false,
            "bytes": 0,
            "fingerprint": null,
        }),
    };

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "falcon": falcon_info,
        "kyber": kyber_info,
        "privacy_mode": state.privacy_mode,
    }))
}

/// Request a certificate.
///
/// Phase F.1: in alpha, this is a local self-signing call backed by the
/// node's FALCON-1024 identity key.  When `state.federation_manager` is
/// `Some` and `threshold_mode_enabled()` returns true, the call is
/// instead dispatched through the threshold-signing coordinator over
/// federated peers.  The caller submits a CSR (raw bytes) and an
/// optional scope; the response is a signed certificate envelope plus
/// the path used (`local` vs `threshold`).
///
/// Params: `{ "csr": "<base64>", "scope"?: "<string>" }`.
async fn handle_request_cert(
    state: &DaemonState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    use base64::Engine;

    let csr_b64 = params
        .get("csr")
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid_params("missing 'csr' parameter (base64-encoded)"))?;
    let csr_bytes = base64::engine::general_purpose::STANDARD
        .decode(csr_b64)
        .map_err(|e| invalid_params(format!("csr base64 decode failed: {e}")))?;
    let scope = params
        .get("scope")
        .and_then(|v| v.as_str())
        .unwrap_or("local")
        .to_string();

    // Phase F.1: federation manager + threshold coordinator wiring.
    // When attached, route through threshold sign coordinator if mode
    // is enabled.  Otherwise we fall back to local self-signing using
    // the node's identity key.
    #[cfg(feature = "intelligence")]
    {
        if let Some(coord) = state.threshold_coordinator.as_ref() {
            if let Some(fed) = state.federation_manager.as_ref() {
                if fed.threshold_mode_enabled().await {
                    // Reconstruct CA fingerprint from local identity.
                    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
                    let ca_fp = match std::fs::read(identity_dir.join("falcon_pubkey.der")) {
                        Ok(pk) if pk.len() >= 32 => {
                            use sha2::{Digest, Sha256};
                            let d: [u8; 32] = Sha256::digest(&pk).into();
                            d
                        }
                        _ => {
                            return Err(internal_error(
                                "threshold mode requires local FALCON public key",
                            ));
                        }
                    };

                    match coord
                        .sign(
                            ca_fp,
                            &csr_bytes,
                            2, // threshold of 2 for alpha
                            std::time::Duration::from_secs(30),
                        )
                        .await
                    {
                        Ok(sig) => {
                            return Ok(serde_json::json!({
                                "node_id": state.node_id,
                                "scope": scope,
                                "path": "threshold",
                                "signature": base64::engine::general_purpose::STANDARD.encode(&sig),
                                "csr_len": csr_bytes.len(),
                                "status": "signed",
                            }));
                        }
                        Err(e) => {
                            return Err(internal_error(format!(
                                "threshold sign failed: {e}"
                            )));
                        }
                    }
                }
            }
        }
    }

    // Local fallback: sign the CSR with the node's FALCON-1024 identity
    // key.  This is the Phase 0 bootstrap path.
    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
    let identity = crate::identity::FalconIdentity::load_or_create(&identity_dir)
        .map_err(|e| internal_error(format!("identity load failed: {e}")))?;
    let signature = <crate::identity::FalconIdentity as hypermesh_lib::NodeSigner>::sign(
        &identity,
        &csr_bytes,
    )
    .map_err(|e| internal_error(format!("local sign failed: {e}")))?;

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "scope": scope,
        "path": "local",
        "signature": base64::engine::general_purpose::STANDARD.encode(&signature),
        "csr_len": csr_bytes.len(),
        "status": "signed",
    }))
}

/// Federation peers: connected CAs and trust levels.
async fn handle_federation(
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let peer_count = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "federation_peers": [],
        "federation_count": 0,
        "network_peers": peer_count,
        "trust_levels": {
            "full": 0,
            "conditional": 0,
            "untrusted": 0,
        },
        "status": "alpha",
        "note": "distributed CA not yet active",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::RpcRequest;

    #[tokio::test]
    async fn test_trustchain_status_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.status", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["cert_type"], "self_signed");
        assert_eq!(result["signing_algorithm"], "FALCON-1024");
        assert_eq!(result["encryption_algorithm"], "Kyber-1024");
        assert_eq!(result["distributed_ca"], false);
        assert_eq!(result["status"], "alpha");
    }

    #[tokio::test]
    async fn test_trustchain_certs_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.certs", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert!(result["certificates"].is_array());
        assert!(result["count"].is_number());
    }

    #[tokio::test]
    async fn test_trustchain_identity_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.identity", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert!(result["falcon"].is_object());
        assert!(result["kyber"].is_object());
        assert_eq!(result["falcon"]["present"], false);
        assert_eq!(result["kyber"]["present"], false);
    }

    #[tokio::test]
    async fn test_trustchain_identity_with_keys() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let identity_dir = dir.path().join("tc-test").join("identity");
        std::fs::create_dir_all(&identity_dir).expect("test: mkdir");

        std::fs::write(identity_dir.join("falcon_pubkey.der"), vec![0xAA; 64])
            .expect("test: write falcon");
        std::fs::write(identity_dir.join("kyber_pubkey.der"), vec![0xBB; 128])
            .expect("test: write kyber");

        let coord = crate::matrix::coordinate::MatrixCoordinate::new(0, 0, 0)
            .expect("test: coord");
        let bc = Arc::new(crate::blockchain::node_chain::NodeBlockchain::new(coord));
        let config = crate::persistence::PersistenceConfig {
            storage_dir: std::path::PathBuf::from("/tmp"),
            ..crate::persistence::PersistenceConfig::default()
        };
        let persistence = Arc::new(
            crate::persistence::PersistenceManager::new(config, "tc-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(crate::ipc::state::DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "tc-test".into(),
            data_dir: dir.path().to_path_buf(),
            privacy_mode: "Private".into(),
            started_at: std::time::Instant::now(),
            shutdown_tx,
            dns_resolver: crate::bootstrap::DnsResolver::default(),
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
        });

        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.identity", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["falcon"]["present"], true);
        assert_eq!(result["falcon"]["bytes"], 64);
        assert_eq!(result["kyber"]["present"], true);
        assert_eq!(result["kyber"]["bytes"], 128);
    }

    #[tokio::test]
    async fn test_trustchain_federation_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.federation", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert!(result["federation_peers"].is_array());
        assert_eq!(result["federation_count"], 0);
        assert_eq!(result["network_peers"], 0);
        assert!(result["trust_levels"].is_object());
    }
}
