// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! TrustChain IPC handlers: CA status, certificates, identity, federation.
//!
//! Alpha: The node uses self-signed certificates for bootstrap (Phase 0). These
//! handlers return the identity key info from disk and structured status for the
//! CA, certificate list, and federation peers.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::RpcError;
use crate::ipc::state::DaemonState;

/// Register TrustChain-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
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
    let identity_dir = state.data_dir.join("identity");
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
    let identity_dir = state.data_dir.join("identity");
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
    let identity_dir = state.data_dir.join("identity");
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
        let identity_dir = dir.path().join("identity");
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
