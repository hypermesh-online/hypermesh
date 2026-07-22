// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase K.2 — IPC capability-enforcement tests.
//!
//! Covers:
//! - Method-to-capability registry mapping (read-only / wallet /
//!   asset-write / admin)
//! - Fail-closed default for unknown methods
//! - Alpha-default inert: when `capability_token_issuer` is `None`
//!   dispatch skips token validation (preserves pre-K.2 behavior)
//! - End-to-end enforcement: valid / insufficient / expired / revoked
//!   tokens produce the right `RpcResponse` shape

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use base64::Engine;
use blockmatrix::auth::{
    Capability, CapabilityToken, CapabilityTokenIssuer, RevocationRegistry,
};
use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::bootstrap::DnsResolver;
use blockmatrix::ipc::handler::{CapabilityContext, RequestHandler};
use blockmatrix::ipc::handlers::auth::CAPABILITY_DENIED;
use blockmatrix::ipc::handlers::capability_registry::required_capability;
use blockmatrix::ipc::handlers::register_all;
use blockmatrix::ipc::protocol::RpcRequest;
use blockmatrix::ipc::state::DaemonState;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::persistence::{PersistenceConfig, PersistenceManager};
use trustchain::FalconIdentity;

// ---------- helpers ----------

async fn build_state(
    issuer: Option<Arc<CapabilityTokenIssuer>>,
    revocation: Arc<RevocationRegistry>,
) -> Arc<DaemonState> {
    let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
    let bc = Arc::new(NodeBlockchain::new(coord));
    let cfg = PersistenceConfig {
        storage_dir: PathBuf::from("/tmp"),
        ..PersistenceConfig::default()
    };
    let persistence = Arc::new(
        PersistenceManager::new(cfg, "k2-test".into())
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
        node_id: "k2-test".into(),
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
            blockmatrix::assets::cross_chain::CrossChainReceiptValidator::new(),
        ),
        capability_token_issuer: issuer,
        revocation_registry: revocation,
        light_sync_manager: None,
            catalog_registry: None,
            inbox_store: None,
    })
}

fn fake_device_pubkey() -> Vec<u8> {
    FalconIdentity::generate().public_key.clone()
}

fn token_to_b64(token: &CapabilityToken) -> String {
    let bytes = serde_json::to_vec(token).expect("test: serialize token");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

// ---------- 1. registry mapping ----------

#[test]
fn test_method_to_capability_registry() {
    // Sample of 20+ representative methods.
    let cases = [
        ("status", Capability::ViewOnly),
        ("ping", Capability::ViewOnly),
        ("blockchain.height", Capability::ViewOnly),
        ("blockchain.block", Capability::ViewOnly),
        ("caesar.balance", Capability::ViewOnly),
        ("caesar.transactions", Capability::ViewOnly),
        ("ngauge.capacity", Capability::ViewOnly),
        ("trustchain.identity", Capability::ViewOnly),
        ("stoq.stats", Capability::ViewOnly),
        ("dns.resolve", Capability::ViewOnly),
        ("asset.list", Capability::ViewOnly),
        ("asset.info", Capability::ViewOnly),
        ("topology.info", Capability::ViewOnly),
        ("share.inbox", Capability::ViewOnly),
        ("auth.list_sessions", Capability::ViewOnly),
        ("system.check_update", Capability::ViewOnly),
        ("chain.lookup_cross_receipt", Capability::ViewOnly),
        ("caesar.transfer", Capability::Wallet),
        ("caesar.staking", Capability::Wallet),
        ("caesar.rewards", Capability::Wallet),
        ("asset.register", Capability::AssetWrite),
        ("store", Capability::AssetWrite),
        ("fetch", Capability::AssetWrite),
        ("share.send", Capability::AssetWrite),
        ("dns.register", Capability::AssetWrite),
        ("domain.register", Capability::AssetWrite),
        ("shutdown", Capability::Admin),
        ("config.set", Capability::Admin),
        ("dns.foundation_grant", Capability::Admin),
        ("auth.create_session", Capability::Admin),
        ("auth.revoke_session", Capability::Admin),
        ("system.apply_update", Capability::Admin),
        ("trustchain.request_cert", Capability::Admin),
        ("gateway.initiate_transfer", Capability::Admin),
    ];
    for (method, expected) in cases {
        assert_eq!(
            required_capability(method),
            expected,
            "method {method} expected {expected:?}",
        );
    }
}

#[test]
fn test_unknown_method_defaults_to_admin() {
    // Fail-closed: anything not explicitly listed requires Admin.
    assert_eq!(
        required_capability("totally.fake.method"),
        Capability::Admin,
    );
    assert_eq!(required_capability(""), Capability::Admin);
    assert_eq!(
        required_capability("future.experiment.beta"),
        Capability::Admin,
    );
}

// ---------- 2. alpha-default inert (no enforcement when issuer None) ----------

#[tokio::test]
async fn test_alpha_default_skips_enforcement_when_issuer_none() {
    let revocation = Arc::new(RevocationRegistry::new());
    let state = build_state(None, revocation).await;
    let mut handler = RequestHandler::new();
    register_all(&mut handler, state);

    assert!(
        !handler.capability_enforcement_enabled(),
        "enforcement must be off when issuer is None",
    );

    // Even an Admin-tier method dispatches without a token.
    let req = RpcRequest::new("status", serde_json::json!({}));
    let resp = handler.dispatch(req).await;
    // status handler runs, no capability_denied error.
    if let Some(err) = &resp.error {
        assert_ne!(err.code, CAPABILITY_DENIED);
    }
}

// ---------- 3. enforcement: valid token + correct scope passes ----------

#[tokio::test]
async fn test_capability_token_validation_in_ipc_middleware() {
    let identity = Arc::new(FalconIdentity::generate());
    let issuer = Arc::new(CapabilityTokenIssuer::new(identity.clone()));
    let revocation = Arc::new(RevocationRegistry::new());
    let state = build_state(Some(issuer.clone()), revocation.clone()).await;
    let mut handler = RequestHandler::new();
    register_all(&mut handler, state);

    assert!(handler.capability_enforcement_enabled());

    // Valid ViewOnly token + ViewOnly request → passes.
    let token = issuer
        .issue(
            fake_device_pubkey(),
            vec![Capability::ViewOnly],
            Duration::from_secs(60),
        )
        .expect("test: issue");
    let req = RpcRequest::new_with_token(
        "status",
        serde_json::json!({}),
        token_to_b64(&token),
    );
    let resp = handler.dispatch(req).await;
    if let Some(err) = &resp.error {
        assert_ne!(err.code, CAPABILITY_DENIED, "unexpected: {}", err.message);
    }

    // ViewOnly token + Admin-required method → CAPABILITY_DENIED.
    let req = RpcRequest::new_with_token(
        "shutdown",
        serde_json::json!({}),
        token_to_b64(&token),
    );
    let resp = handler.dispatch(req).await;
    let err = resp.error.expect("expected denial");
    assert_eq!(err.code, CAPABILITY_DENIED);
    assert!(err.message.contains("does not grant"));
}

// ---------- 4. revoked session rejected ----------

#[tokio::test]
async fn test_capability_token_revoked_session_rejected() {
    let identity = Arc::new(FalconIdentity::generate());
    let issuer = Arc::new(CapabilityTokenIssuer::new(identity.clone()));
    let revocation = Arc::new(RevocationRegistry::new());
    let state =
        build_state(Some(issuer.clone()), revocation.clone()).await;
    let mut handler = RequestHandler::new();
    register_all(&mut handler, state);

    let token = issuer
        .issue(
            fake_device_pubkey(),
            vec![Capability::Admin],
            Duration::from_secs(60),
        )
        .expect("test: issue");

    // Revoke before dispatch.
    revocation.revoke(token.session_id).await;

    let req = RpcRequest::new_with_token(
        "status",
        serde_json::json!({}),
        token_to_b64(&token),
    );
    let resp = handler.dispatch(req).await;
    let err = resp.error.expect("expected revocation rejection");
    assert_eq!(err.code, CAPABILITY_DENIED);
    assert!(err.message.contains("revoked"));
}

// ---------- 5. expired token rejected ----------

#[tokio::test]
async fn test_capability_token_expired_rejected() {
    let identity = Arc::new(FalconIdentity::generate());
    let issuer = Arc::new(CapabilityTokenIssuer::new(identity.clone()));
    let revocation = Arc::new(RevocationRegistry::new());
    let state = build_state(Some(issuer.clone()), revocation.clone()).await;
    let mut handler = RequestHandler::new();
    register_all(&mut handler, state);

    // Manually craft an expired-but-otherwise-valid token by issuing
    // with TTL=1ns and waiting a few ms.
    let token = issuer
        .issue(
            fake_device_pubkey(),
            vec![Capability::Admin],
            Duration::from_nanos(1),
        )
        .expect("test: issue");
    // sleep enough that valid_until is comfortably in the past.
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(token.is_expired_at(SystemTime::now()));

    let req = RpcRequest::new_with_token(
        "status",
        serde_json::json!({}),
        token_to_b64(&token),
    );
    let resp = handler.dispatch(req).await;
    let err = resp.error.expect("expected expiry rejection");
    assert_eq!(err.code, CAPABILITY_DENIED);
    assert!(err.message.contains("expired"));
}

// ---------- 6. missing token rejected when enforcement enabled ----------

#[tokio::test]
async fn test_missing_token_rejected_when_enforcement_enabled() {
    let identity = Arc::new(FalconIdentity::generate());
    let issuer = Arc::new(CapabilityTokenIssuer::new(identity.clone()));
    let revocation = Arc::new(RevocationRegistry::new());
    let state = build_state(Some(issuer), revocation).await;
    let mut handler = RequestHandler::new();
    register_all(&mut handler, state);

    let req = RpcRequest::new("status", serde_json::json!({}));
    let resp = handler.dispatch(req).await;
    let err = resp.error.expect("expected missing-token rejection");
    assert_eq!(err.code, CAPABILITY_DENIED);
    assert!(err.message.contains("no capability_token"));
}

// ---------- 7. CapabilityContext direct validation ----------

#[tokio::test]
async fn test_capability_context_validate_paths() {
    let identity = Arc::new(FalconIdentity::generate());
    let issuer = Arc::new(CapabilityTokenIssuer::new(identity.clone()));
    let revocation = Arc::new(RevocationRegistry::new());
    let ctx = CapabilityContext::new(&issuer, revocation.clone());

    let token = issuer
        .issue(
            fake_device_pubkey(),
            vec![Capability::Wallet],
            Duration::from_secs(60),
        )
        .expect("test: issue");
    let b64 = token_to_b64(&token);

    // Wallet-required method passes.
    ctx.validate(&b64, Capability::Wallet)
        .await
        .expect("wallet must pass");

    // ViewOnly required passes too because Wallet... wait — Capability
    // does not implement subset relations beyond Admin.
    // Wallet does NOT grant ViewOnly. Confirm.
    let res = ctx.validate(&b64, Capability::ViewOnly).await;
    assert!(res.is_err(), "Wallet token should NOT grant ViewOnly");

    // Admin-required denied.
    let res = ctx.validate(&b64, Capability::Admin).await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, CAPABILITY_DENIED);
}
