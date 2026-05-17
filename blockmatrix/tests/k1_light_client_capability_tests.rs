// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase K.1 — Light client + capability token integration tests.
//!
//! Covers:
//! - `HeaderSyncManager` chain ingestion and orphan rejection
//! - `WitnessedProofVerifier` accept/reject paths
//! - `CapabilityToken` round-trip, signature tampering, expiry
//! - `Capability` allow rules (Admin superset, ViewOnly denies write)
//! - `auth.create_session` IPC happy path + alpha-default inert
//! - `SessionAudit` block entry recorded on chain

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use blockmatrix::auth::{
    Capability, CapabilityToken, CapabilityTokenIssuer, RevocationRegistry,
};
use blockmatrix::blockchain::block::BlockHeader;
use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::bootstrap::DnsResolver;
use blockmatrix::ipc::handler::RequestHandler;
use blockmatrix::ipc::handlers::register_all;
use blockmatrix::ipc::protocol::RpcRequest;
use blockmatrix::ipc::state::DaemonState;
use blockmatrix::light_client::{HeaderSyncManager, LightClientError, WitnessedProofVerifier};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::persistence::{PersistenceConfig, PersistenceManager};
use trustchain::FalconIdentity;

// ---------- shared fixtures ----------

fn header(index: u64, hash: &str, prev: &str, entries_seed: &[u8]) -> BlockHeader {
    BlockHeader {
        index,
        hash: hash.to_string(),
        previous_hash: prev.to_string(),
        entries_hash: *blake3::hash(entries_seed).as_bytes(),
        entry_count: 1,
    }
}

async fn build_state_with_issuer(
    issuer: Option<Arc<CapabilityTokenIssuer>>,
) -> Arc<DaemonState> {
    let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
    let bc = Arc::new(NodeBlockchain::new(coord));
    let cfg = PersistenceConfig {
        storage_dir: PathBuf::from("/tmp"),
        ..PersistenceConfig::default()
    };
    let persistence = Arc::new(
        PersistenceManager::new(cfg, "k1-test".into())
            .await
            .expect("test: persistence"),
    );
    let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
    let dns = DnsResolver::default();

    Arc::new(DaemonState {
        blockchain: bc,
        persistence,
        network: None,
        shard_store: Arc::new(ShardStore::new()),
        shard_transport: None,
        coordinate: coord,
        node_id: "k1-test".into(),
        data_dir: PathBuf::from("/tmp"),
        privacy_mode: "Private".into(),
        started_at: Instant::now(),
        shutdown_tx,
        dns_resolver: dns,
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
            blockmatrix::assets::cross_chain::CrossChainReceiptValidator::new(),
        ),
        capability_token_issuer: issuer,
        revocation_registry: Arc::new(RevocationRegistry::new()),
        light_sync_manager: None,
            catalog_registry: None,
    })
}

// ---------- HeaderSyncManager ----------

#[tokio::test]
async fn test_header_sync_manager_ingest_chains() {
    let mgr = HeaderSyncManager::new();

    let mut prev_hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
    for i in 0..100u64 {
        let hash = format!("blk-{:064}", i);
        let h = header(i, &hash, &prev_hash, &i.to_le_bytes());
        mgr.ingest_header(h).await.expect("ingest must succeed");
        prev_hash = hash;
    }

    let tip = mgr.chain_tip().await.expect("tip present");
    assert_eq!(tip.index, 99);
    assert_eq!(mgr.header_count().await, 100);
    assert_eq!(mgr.tip_height().await, 100);
}

#[tokio::test]
async fn test_header_sync_manager_rejects_orphan() {
    let mgr = HeaderSyncManager::new();
    mgr.ingest_header(header(0, "g", "0", b"genesis"))
        .await
        .expect("genesis");

    // Orphan: previous_hash points at something other than "g".
    let bad = header(1, "h1", "wrong-prev", b"one");
    let err = mgr.ingest_header(bad).await.unwrap_err();
    assert!(matches!(err, LightClientError::OrphanHeader { .. }));

    // Out-of-order: skip from index 0 to index 5.
    let skipped = header(5, "h5", "g", b"five");
    let err2 = mgr.ingest_header(skipped).await.unwrap_err();
    assert!(matches!(err2, LightClientError::IndexOutOfOrder { .. }));
}

// ---------- WitnessedProofVerifier ----------

#[tokio::test]
async fn test_witnessed_proof_verifier_accepts_valid() {
    let mgr = Arc::new(HeaderSyncManager::new());
    mgr.ingest_header(header(0, "g", "0", b"gen"))
        .await
        .expect("ingest genesis");
    mgr.ingest_header(header(1, "h1", "g", b"one"))
        .await
        .expect("ingest one");

    let verifier = WitnessedProofVerifier::new(mgr.clone());
    // K.1 alpha: header presence = witness.
    let proof_hash = blake3::hash(b"some-state-proof").as_bytes().to_vec();
    let ok = verifier.verify_proof(1, &proof_hash).await.expect("verify");
    assert!(ok);

    // Tip variant.
    let ok2 = verifier
        .verify_proof_at_tip(&proof_hash)
        .await
        .expect("verify at tip");
    assert!(ok2);
}

#[tokio::test]
async fn test_witnessed_proof_verifier_rejects_unknown_block() {
    let mgr = Arc::new(HeaderSyncManager::new());
    mgr.ingest_header(header(0, "g", "0", b"gen"))
        .await
        .expect("ingest");

    let verifier = WitnessedProofVerifier::new(mgr.clone());
    let err = verifier.verify_proof(99, b"x").await.unwrap_err();
    assert!(matches!(err, LightClientError::HeaderNotFound(99)));
}

// ---------- CapabilityToken ----------

#[test]
fn test_capability_token_round_trip() {
    let id = Arc::new(FalconIdentity::generate());
    let issuer = CapabilityTokenIssuer::new(id.clone());

    let device = FalconIdentity::generate().public_key.clone();
    let token = issuer
        .issue(device.clone(), vec![Capability::Wallet], Duration::from_secs(60))
        .expect("issue");
    assert!(token.verify(&id.public_key));

    // Serialize → deserialize → still verifies.
    let json = serde_json::to_vec(&token).expect("serialize");
    let back: CapabilityToken = serde_json::from_slice(&json).expect("deserialize");
    assert!(back.verify(&id.public_key));
    assert_eq!(back.session_id, token.session_id);
}

#[test]
fn test_capability_token_signature_tampering_detected() {
    let id = Arc::new(FalconIdentity::generate());
    let issuer = CapabilityTokenIssuer::new(id.clone());
    let mut token = issuer
        .issue(
            FalconIdentity::generate().public_key.clone(),
            vec![Capability::ViewOnly],
            Duration::from_secs(60),
        )
        .expect("issue");

    // Flip first byte of signature.
    token.signature[0] ^= 0xFF;
    assert!(!token.verify(&id.public_key));
}

#[test]
fn test_capability_token_expiry() {
    let now = SystemTime::now();
    let past = now - Duration::from_secs(120);
    let token = CapabilityToken::new_unsigned(
        vec![1u8; 8],
        vec![Capability::ViewOnly],
        past,
        past + Duration::from_secs(1), // expired 119s ago
        vec![2u8; 8],
    );
    assert!(token.is_expired());
}

#[test]
fn test_capability_token_admin_implies_all() {
    let now = SystemTime::now();
    let token = CapabilityToken::new_unsigned(
        vec![1u8; 8],
        vec![Capability::Admin],
        now,
        now + Duration::from_secs(60),
        vec![2u8; 8],
    );
    assert!(token.allows(&Capability::ViewOnly));
    assert!(token.allows(&Capability::Wallet));
    assert!(token.allows(&Capability::AssetWrite));
    assert!(token.allows(&Capability::Admin));
}

#[test]
fn test_capability_token_view_only_denies_write() {
    let now = SystemTime::now();
    let token = CapabilityToken::new_unsigned(
        vec![1u8; 8],
        vec![Capability::ViewOnly],
        now,
        now + Duration::from_secs(60),
        vec![2u8; 8],
    );
    assert!(token.allows(&Capability::ViewOnly));
    assert!(!token.allows(&Capability::AssetWrite));
    assert!(!token.allows(&Capability::Wallet));
    assert!(!token.allows(&Capability::Admin));
}

// ---------- IPC: auth.create_session ----------

#[tokio::test]
async fn test_auth_create_session_returns_signed_token() {
    let id = Arc::new(FalconIdentity::generate());
    let issuer = Arc::new(CapabilityTokenIssuer::new(id.clone()));
    let state = build_state_with_issuer(Some(issuer)).await;

    let mut handler = RequestHandler::new();
    register_all(&mut handler, state.clone());

    let device_pubkey = FalconIdentity::generate().public_key.clone();
    let req = RpcRequest::new(
        "auth.create_session",
        serde_json::json!({
            "device_pubkey": hex::encode(&device_pubkey),
            "requested_capabilities": ["viewonly", "wallet"],
            "ttl_secs": 60,
        }),
    );
    let resp = handler.dispatch(req).await;
    assert!(resp.error.is_none(), "expected success, got: {:?}", resp.error);
    let result = resp.result.expect("result present");

    // Reconstruct token and verify.
    let token_json = result.get("token").expect("token present").clone();
    let token: CapabilityToken =
        serde_json::from_value(token_json).expect("deserialize token");
    assert!(token.verify(&id.public_key));
    assert_eq!(token.device_pubkey, device_pubkey);
    assert!(token.allows(&Capability::ViewOnly));
    assert!(token.allows(&Capability::Wallet));
}

#[tokio::test]
async fn test_auth_create_session_rejects_when_not_configured() {
    // capability_token_issuer = None => alpha-default inert.
    let state = build_state_with_issuer(None).await;
    let mut handler = RequestHandler::new();
    register_all(&mut handler, state);

    let device_pubkey = vec![0u8; 16];
    let req = RpcRequest::new(
        "auth.create_session",
        serde_json::json!({
            "device_pubkey": hex::encode(&device_pubkey),
            "requested_capabilities": ["viewonly"],
            "ttl_secs": 60,
        }),
    );
    let resp = handler.dispatch(req).await;
    let err = resp.error.expect("expected inert rejection");
    assert!(
        err.message.contains("auth not configured")
            && err.message.contains("alpha-default inert"),
        "expected alpha-default inert message, got: {}",
        err.message
    );
}

// ---------- IPC: chain audit ----------

#[tokio::test]
async fn test_session_audit_recorded_on_chain() {
    let id = Arc::new(FalconIdentity::generate());
    let issuer = Arc::new(CapabilityTokenIssuer::new(id.clone()));
    let state = build_state_with_issuer(Some(issuer)).await;

    let height_before = state.blockchain.get_height().await;

    let mut handler = RequestHandler::new();
    register_all(&mut handler, state.clone());

    let device_pubkey = FalconIdentity::generate().public_key.clone();
    let req = RpcRequest::new(
        "auth.create_session",
        serde_json::json!({
            "device_pubkey": hex::encode(&device_pubkey),
            "requested_capabilities": ["viewonly"],
            "ttl_secs": 60,
        }),
    );
    let resp = handler.dispatch(req).await;
    assert!(resp.error.is_none(), "session creation failed");

    let height_after = state.blockchain.get_height().await;
    assert!(
        height_after > height_before,
        "expected chain to grow after session creation (before={}, after={})",
        height_before,
        height_after,
    );

    // Inspect the latest block — should include a SessionAudit-encoded entry.
    let head = state.blockchain.get_head().await.expect("head present");
    assert!(!head.entries.is_empty());

    // The payload was JSON-serialized into StoragePointer::Local.
    let entry = &head.entries[0];
    if let blockmatrix::blockchain::block::StoragePointer::Local { path } =
        &entry.storage_pointer
    {
        assert!(
            path.contains("\"version\":1") || path.contains("session_id"),
            "expected SessionAudit JSON payload, got: {}",
            path
        );
    } else {
        panic!("expected StoragePointer::Local for SessionAudit entry");
    }

    // Suppress unused-import warnings for SystemTime/UNIX_EPOCH on this branch.
    let _ = (SystemTime::now(), UNIX_EPOCH);
}
