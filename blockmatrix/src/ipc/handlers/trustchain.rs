// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! TrustChain IPC handlers: CA status, certificates, identity, federation.
//!
//! Alpha: The node uses self-signed certificates for bootstrap (Phase 0). These
//! handlers return the identity key info from disk and structured status for the
//! CA, certificate list, and federation peers.
//!
//! Phase M.2.5 — handlers parse real X.509 DER from disk and surface full
//! cryptographic metadata (validity window, fingerprints, key usage, SANs,
//! extensions) for dashboards. No cosmetic placeholders.

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

/// List issued certificates. Reads the node's own `node_cert.der` from disk
/// and returns fully parsed X.509 metadata. If the cert is missing, returns
/// an empty list (NO fake placeholder row). If present but unparseable,
/// surfaces an `error` field instead of crashing.
fn handle_certs(state: &DaemonState) -> Result<serde_json::Value, RpcError> {
    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
    let cert_path = identity_dir.join("node_cert.der");
    let cert_path_str = cert_path.to_string_lossy().to_string();

    let bytes = match std::fs::read(&cert_path) {
        Ok(b) => b,
        Err(_) => {
            return Ok(serde_json::json!({
                "node_id": state.node_id,
                "certificates": [],
                "total": 0,
                "status": "alpha",
            }));
        }
    };

    match super::trustchain_x509::parse_cert_to_json(&bytes, &cert_path_str) {
        Ok(value) => Ok(serde_json::json!({
            "node_id": state.node_id,
            "certificates": [value],
            "total": 1,
            "status": "alpha",
        })),
        Err(msg) => Ok(serde_json::json!({
            "node_id": state.node_id,
            "certificates": [],
            "total": 0,
            "status": "alpha",
            "error": msg,
        })),
    }
}

/// Identity key summary: public key sizes, BLAKE3 fingerprints (HyperMesh
/// canonical), and `created_at` derived from the node cert's `not_before` if
/// available.
fn handle_identity(state: &DaemonState) -> Result<serde_json::Value, RpcError> {
    let identity_dir = state.data_dir.join(&state.node_id).join("identity");
    let falcon_pk_path = identity_dir.join("falcon_pubkey.der");
    let kyber_pk_path = identity_dir.join("kyber_pubkey.der");
    let cert_path = identity_dir.join("node_cert.der");

    let falcon_info = match std::fs::read(&falcon_pk_path) {
        Ok(pk) => serde_json::json!({
            "present": true,
            "bytes": pk.len(),
            "fingerprint": blake3::hash(&pk).to_hex().to_string(),
            "key_algorithm": "FALCON-1024",
        }),
        Err(_) => serde_json::json!({
            "present": false,
            "bytes": 0,
            "fingerprint": null,
            "key_algorithm": "FALCON-1024",
        }),
    };

    let kyber_info = match std::fs::read(&kyber_pk_path) {
        Ok(pk) => serde_json::json!({
            "present": true,
            "bytes": pk.len(),
            "fingerprint": blake3::hash(&pk).to_hex().to_string(),
            "key_algorithm": "Kyber-1024",
        }),
        Err(_) => serde_json::json!({
            "present": false,
            "bytes": 0,
            "fingerprint": null,
            "key_algorithm": "Kyber-1024",
        }),
    };

    // created_at: pull from cert not_before when the cert exists and parses.
    let created_at = std::fs::read(&cert_path)
        .ok()
        .and_then(|der| {
            x509_parser::parse_x509_certificate(&der)
                .ok()
                .map(|(_, cert)| cert.tbs_certificate.validity.not_before.timestamp())
        });

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "falcon": falcon_info,
        "kyber": kyber_info,
        "created_at": created_at,
        "privacy_mode": state.privacy_mode,
        "status": "alpha",
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
///
/// Shape matches the UI `FederationInfo` contract: `peers[]`, `total_peers`,
/// `network_peers`, `trust_levels{}`. Each peer (when populated by Phase F
/// federation) carries `{node_id, trust_level, joined_at, fingerprint}`; the
/// array stays empty until distributed CA is active.
async fn handle_federation(
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let network_peers = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "peers": [],
        "total_peers": 0,
        "network_peers": network_peers,
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

    /// Generate a self-signed DER cert via rcgen for parsing tests.
    fn make_test_cert_der(common_name: &str) -> Vec<u8> {
        let cert_key = rcgen::generate_simple_self_signed(vec![common_name.to_string()])
            .expect("test: generate self-signed cert");
        cert_key
            .cert
            .der()
            .as_ref()
            .to_vec()
    }

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
    async fn test_trustchain_certs_missing_cert_returns_empty() {
        // test_state() points at /tmp/test-node/identity which (almost
        // certainly) has no node_cert.der — the handler must return an
        // empty list with NO fake placeholder row, no panic, no error.
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.certs", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        let certs = result["certificates"].as_array().expect("test: array");
        // It is acceptable for the path to actually exist in /tmp if a
        // previous test wrote there. In that case it must parse cleanly
        // (total>=1) — but it must never be a fake/placeholder row.
        if certs.is_empty() {
            assert_eq!(result["total"], 0);
        } else {
            // Real parsed cert — must have the rich fields.
            assert!(certs[0]["fingerprint_blake3"].is_string());
            assert!(certs[0]["valid_from"].is_string());
            assert!(certs[0]["valid_to"].is_string());
        }
        assert_eq!(result["status"], "alpha");
    }

    #[tokio::test]
    async fn test_trustchain_certs_real_x509_parse() {
        // Build a temporary state pointed at a fresh tempdir, write a
        // real self-signed cert, and assert all the rich fields parse.
        let dir = tempfile::tempdir().expect("test: tempdir");
        let identity_dir = dir.path().join("tc-parse-test").join("identity");
        std::fs::create_dir_all(&identity_dir).expect("test: mkdir");

        let der = make_test_cert_der("parse-test.hypermesh");
        std::fs::write(identity_dir.join("node_cert.der"), &der)
            .expect("test: write cert");

        let state = build_test_state_at(dir.path(), "tc-parse-test").await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.certs", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["total"], 1);
        let cert = &result["certificates"][0];

        // Identity fields
        assert!(cert["id"].is_string());
        assert_eq!(
            cert["id"].as_str().expect("test: id str").len(),
            64,
            "BLAKE3 fingerprint must be 64 hex chars"
        );
        assert!(cert["subject"].is_string());
        assert!(cert["issuer"].is_string());

        // Validity is RFC3339 — must start with a 4-digit year.
        let vf = cert["valid_from"].as_str().expect("test: valid_from str");
        let vt = cert["valid_to"].as_str().expect("test: valid_to str");
        assert!(vf.len() >= 20 && &vf[4..5] == "-", "valid_from RFC3339: {vf}");
        assert!(vt.len() >= 20 && &vt[4..5] == "-", "valid_to RFC3339: {vt}");

        // Active status (cert was just generated)
        assert_eq!(cert["status"], "active");

        // Fingerprints — distinct hex digests
        let blake3_hex = cert["fingerprint_blake3"]
            .as_str()
            .expect("test: blake3 str");
        let sha256_hex = cert["fingerprint_sha256"]
            .as_str()
            .expect("test: sha256 str");
        assert_eq!(blake3_hex.len(), 64);
        assert_eq!(sha256_hex.len(), 64);
        assert_ne!(blake3_hex, sha256_hex);
        assert_eq!(cert["id"], cert["fingerprint_blake3"]);

        // Arrays must exist (may be empty for a vanilla rcgen cert).
        assert!(cert["key_usage"].is_array());
        assert!(cert["extended_key_usage"].is_array());
        assert!(cert["subject_alt_names"].is_array());
        assert!(cert["extensions"].is_array());

        // SAN should contain our DNS name
        let sans = cert["subject_alt_names"]
            .as_array()
            .expect("test: sans array");
        assert!(
            sans.iter().any(|s| s.as_str() == Some("DNS:parse-test.hypermesh")),
            "expected SAN DNS:parse-test.hypermesh, got {:?}",
            sans
        );

        // Algorithm fields present
        assert!(cert["signature_algorithm"].is_string());
        assert!(cert["key_algorithm"].is_string());

        // Path matches what we wrote
        assert!(cert["path"]
            .as_str()
            .expect("test: path str")
            .ends_with("node_cert.der"));
    }

    #[tokio::test]
    async fn test_trustchain_certs_corrupt_returns_empty_with_error() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let identity_dir = dir.path().join("tc-bad-test").join("identity");
        std::fs::create_dir_all(&identity_dir).expect("test: mkdir");
        std::fs::write(identity_dir.join("node_cert.der"), b"not a cert")
            .expect("test: write garbage");

        let state = build_test_state_at(dir.path(), "tc-bad-test").await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.certs", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["total"], 0);
        assert!(result["certificates"].as_array().expect("test: arr").is_empty());
        assert!(result["error"].is_string(), "error message must be surfaced");
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
        // Default test_state() points at /tmp/test-node and almost
        // certainly has no keys — but tests can leak. Either way the
        // flat shape must hold.
        assert!(result["falcon"]["present"].is_boolean());
        assert!(result["kyber"]["present"].is_boolean());
        assert_eq!(result["falcon"]["key_algorithm"], "FALCON-1024");
        assert_eq!(result["kyber"]["key_algorithm"], "Kyber-1024");
        assert_eq!(result["status"], "alpha");
    }

    #[tokio::test]
    async fn test_trustchain_identity_with_keys_and_cert() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let identity_dir = dir.path().join("tc-test").join("identity");
        std::fs::create_dir_all(&identity_dir).expect("test: mkdir");

        std::fs::write(identity_dir.join("falcon_pubkey.der"), vec![0xAA; 64])
            .expect("test: write falcon");
        std::fs::write(identity_dir.join("kyber_pubkey.der"), vec![0xBB; 128])
            .expect("test: write kyber");

        let der = make_test_cert_der("identity-test.hypermesh");
        std::fs::write(identity_dir.join("node_cert.der"), &der)
            .expect("test: write cert");

        let state = build_test_state_at(dir.path(), "tc-test").await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.identity", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["falcon"]["present"], true);
        assert_eq!(result["falcon"]["bytes"], 64);
        assert_eq!(
            result["falcon"]["fingerprint"]
                .as_str()
                .expect("test: falcon fp str")
                .len(),
            64,
            "BLAKE3 fingerprint must be full 32-byte hex (64 chars), not first-32-bytes truncated"
        );

        assert_eq!(result["kyber"]["present"], true);
        assert_eq!(result["kyber"]["bytes"], 128);
        assert_eq!(
            result["kyber"]["fingerprint"]
                .as_str()
                .expect("test: kyber fp str")
                .len(),
            64,
        );

        // created_at must come from cert not_before (unix timestamp).
        assert!(
            result["created_at"].is_i64() || result["created_at"].is_u64(),
            "created_at must be an integer unix timestamp, got {:?}",
            result["created_at"]
        );
    }

    #[tokio::test]
    async fn test_trustchain_federation_keys() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("trustchain.federation", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        // New UI-contract keys present
        assert!(result["peers"].is_array(), "peers[] must be present");
        assert_eq!(result["total_peers"], 0);
        assert_eq!(result["network_peers"], 0);
        assert!(result["trust_levels"].is_object());
        assert_eq!(result["status"], "alpha");

        // Legacy keys absent
        assert!(
            result.get("federation_peers").is_none(),
            "legacy `federation_peers` must be removed"
        );
        assert!(
            result.get("federation_count").is_none(),
            "legacy `federation_count` must be removed"
        );
    }

    // -----------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------

    async fn build_test_state_at(
        data_dir: &std::path::Path,
        node_id: &str,
    ) -> Arc<crate::ipc::state::DaemonState> {
        let coord = crate::matrix::coordinate::MatrixCoordinate::new(0, 0, 0)
            .expect("test: coord");
        let bc = Arc::new(crate::blockchain::node_chain::NodeBlockchain::new(coord));
        let config = crate::persistence::PersistenceConfig {
            storage_dir: std::path::PathBuf::from("/tmp"),
            ..crate::persistence::PersistenceConfig::default()
        };
        let persistence = Arc::new(
            crate::persistence::PersistenceManager::new(config, node_id.to_string())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        Arc::new(crate::ipc::state::DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: node_id.to_string(),
            data_dir: data_dir.to_path_buf(),
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
            inbox_store: None,
        })
    }
}
