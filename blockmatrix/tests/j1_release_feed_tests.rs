// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase J.1 — Release feed + IPC update + block-format V2 reservation tests.
//!
//! Eleven scenarios exercising:
//!   1.  Release feed entry JSON round-trip stability
//!   2.  FALCON-1024 signature verification (positive + negative paths)
//!   3.  Subscriber `latest_for_channel` selection by semver
//!   4.  Alpha-default inert when no foundation pubkey is configured
//!   5–6. `system.check_update` IPC behavior in both states
//!   7.  Block format V2 magic recognized via `deserialize_block_verified`
//!   8.  Block format V1 still works (backward compat)
//!   9.  Legacy raw-bincode still works (backward compat)
//!  10. Protocol-version major-mismatch rejected by IPC server
//!  11. Protocol-version minor-mismatch accepted (forward-compat)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use blockmatrix::release_feed::{
    ReleaseChannel, ReleaseFeedEntry, ReleaseFeedError, ReleaseFeedSubscriber,
};
use hypermesh_lib::NodeSigner;
use trustchain::FalconIdentity;

// ── Helpers ────────────────────────────────────────────────────────────

fn signed_entry(
    signer: &FalconIdentity,
    version: &str,
    channel: ReleaseChannel,
) -> ReleaseFeedEntry {
    let mut hashes = HashMap::new();
    hashes.insert(
        "x86_64-unknown-linux-musl".to_string(),
        "ab".repeat(32),
    );
    hashes.insert(
        "aarch64-apple-darwin".to_string(),
        "cd".repeat(32),
    );
    let mut entry = ReleaseFeedEntry {
        version: version.to_string(),
        channel,
        binary_hashes: hashes,
        release_notes_url: format!("https://release.hypermesh.online/{}", version),
        signed_by: signer.public_key.clone(),
        signature: Vec::new(),
        requires_min_version: None,
        breaking_changes: false,
        issued_at: SystemTime::now(),
    };
    entry.signature = signer
        .sign(&entry.signing_payload())
        .expect("test: FALCON sign");
    entry
}

// ── 1. JSON round-trip stability ───────────────────────────────────────

#[test]
fn test_release_feed_entry_round_trip() {
    let foundation = FalconIdentity::generate();
    let entry = signed_entry(&foundation, "0.1.0", ReleaseChannel::Stable);
    let json = serde_json::to_string(&entry).expect("test: serialize");
    let back: ReleaseFeedEntry =
        serde_json::from_str(&json).expect("test: deserialize");
    assert_eq!(back.version, entry.version);
    assert_eq!(back.channel, entry.channel);
    assert_eq!(back.binary_hashes, entry.binary_hashes);
    assert_eq!(back.signature, entry.signature);
    // After round-trip the signature must still verify
    back.verify(&foundation.public_key)
        .expect("test: round-tripped signature still verifies");
}

// ── 2. Signature verification ──────────────────────────────────────────

#[test]
fn test_release_feed_signature_verification() {
    let foundation = FalconIdentity::generate();
    let other = FalconIdentity::generate();

    // Positive: signs and verifies.
    let entry = signed_entry(&foundation, "0.2.0", ReleaseChannel::Beta);
    entry
        .verify(&foundation.public_key)
        .expect("test: positive verify");

    // Negative — wrong key.
    assert!(matches!(
        entry.verify(&other.public_key),
        Err(ReleaseFeedError::InvalidSignature)
    ));

    // Negative — tampered signature.
    let mut tampered_sig = entry.clone();
    if !tampered_sig.signature.is_empty() {
        let mid = tampered_sig.signature.len() / 2;
        tampered_sig.signature[mid] ^= 0xff;
    }
    assert!(matches!(
        tampered_sig.verify(&foundation.public_key),
        Err(ReleaseFeedError::InvalidSignature)
    ));

    // Negative — tampered payload (mutate version after signing).
    let mut tampered_payload = entry.clone();
    tampered_payload.version = "9.9.9".to_string();
    assert!(matches!(
        tampered_payload.verify(&foundation.public_key),
        Err(ReleaseFeedError::InvalidSignature)
    ));
}

// ── 3. Subscriber latest_for_channel ───────────────────────────────────

#[tokio::test]
async fn test_release_feed_subscriber_latest_for_channel() {
    let foundation = FalconIdentity::generate();
    let sub = ReleaseFeedSubscriber::with_foundation_pubkey(foundation.public_key.clone());

    let stable_old = signed_entry(&foundation, "0.1.0", ReleaseChannel::Stable);
    let stable_new = signed_entry(&foundation, "0.5.0", ReleaseChannel::Stable);
    let beta = signed_entry(&foundation, "0.4.0", ReleaseChannel::Beta);

    sub.ingest(stable_old).await.expect("test: stable_old");
    sub.ingest(stable_new).await.expect("test: stable_new");
    sub.ingest(beta).await.expect("test: beta");

    let latest_stable = sub
        .latest_for_channel(ReleaseChannel::Stable)
        .await
        .expect("test: stable latest");
    let latest_beta = sub
        .latest_for_channel(ReleaseChannel::Beta)
        .await
        .expect("test: beta latest");
    let latest_nightly = sub.latest_for_channel(ReleaseChannel::Nightly).await;

    assert_eq!(latest_stable.version, "0.5.0");
    assert_eq!(latest_beta.version, "0.4.0");
    assert!(latest_nightly.is_none());
}

// ── 4. Alpha-default inert ─────────────────────────────────────────────

#[tokio::test]
async fn test_release_feed_subscriber_rejects_without_foundation_pubkey() {
    let foundation = FalconIdentity::generate();
    let entry = signed_entry(&foundation, "0.1.0", ReleaseChannel::Stable);
    let sub = ReleaseFeedSubscriber::new();
    let err = sub.ingest(entry).await.expect_err("test: should reject");
    assert!(matches!(err, ReleaseFeedError::NotConfigured));
    assert_eq!(sub.cached_count().await, 0);
}

// ── 5–6. system.check_update IPC ───────────────────────────────────────
//
// These tests verify the IPC handler's logic path via the internal
// `RequestHandler` rather than over a Unix socket; the socket layer is
// covered by `test_protocol_version_*` below.

mod system_ipc {
    use super::*;
    // Re-export so we can construct a state directly.
    use blockmatrix::blockchain::node_chain::NodeBlockchain;
    use blockmatrix::bootstrap::DnsResolver;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::network::shard_store::ShardStore;
    use blockmatrix::persistence::{PersistenceConfig, PersistenceManager};
    use std::path::PathBuf;
    use std::time::Instant;

    // We want to call into the IPC handler module without going
    // through Unix sockets. The handler is `pub mod system` under
    // `ipc::handlers`, which `register`s methods on a
    // `RequestHandler`. Use the public re-exports.
    use blockmatrix::ipc::handler::RequestHandler;
    use blockmatrix::ipc::handlers::system as system_handlers;
    use blockmatrix::ipc::protocol::RpcRequest;
    use blockmatrix::ipc::state::DaemonState;

    async fn make_state(
        subscriber: Option<Arc<ReleaseFeedSubscriber>>,
    ) -> Arc<DaemonState> {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: PathBuf::from("/tmp"),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "j1-test".into())
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
            node_id: "j1-test".into(),
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
            ngauge_bridge: None,
            #[cfg(feature = "intelligence")]
            federation_manager: None,
            #[cfg(feature = "intelligence")]
            threshold_coordinator: None,
            transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
            release_feed_subscriber: subscriber,
            receipt_validator: Arc::new(
                blockmatrix::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
            capability_token_issuer: None,
            revocation_registry: Arc::new(blockmatrix::auth::RevocationRegistry::new()),
            light_sync_manager: None,
            catalog_registry: None,
            inbox_store: None,
        })
    }

    #[tokio::test]
    async fn test_check_update_when_update_available() {
        let foundation = FalconIdentity::generate();
        let sub = Arc::new(ReleaseFeedSubscriber::with_foundation_pubkey(
            foundation.public_key.clone(),
        ));
        let entry = signed_entry(&foundation, "99.0.0", ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");

        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        system_handlers::register(&mut h, &state);

        let req = RpcRequest::new("system.check_update", serde_json::json!({}));
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none(), "expected ok, got {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["up_to_date"], false);
        assert_eq!(result["available_version"], "99.0.0");
        assert!(result["release_notes_url"].is_string());
    }

    #[tokio::test]
    async fn test_check_update_when_up_to_date() {
        // No subscriber → up_to_date with note.
        let state = make_state(None).await;
        let mut h = RequestHandler::new();
        system_handlers::register(&mut h, &state);
        let req = RpcRequest::new("system.check_update", serde_json::json!({}));
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["up_to_date"], true);
    }
}

// ── 7–9. Block format V1 / V2 / legacy backward-compat ────────────────
//
// Phase J.1 reservation: V2 magic recognized but identical schema to V1.

#[test]
fn test_block_format_v2_magic_recognized() {
    use blockmatrix::blockchain::block::Block;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::persistence::blockchain_storage::{
        test_deserialize, test_serialize_v2,
    };

    let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
    let block = Block::genesis(coord);
    let bytes = test_serialize_v2(&block).expect("test: serialize V2");
    // V2 magic prefix
    assert_eq!(&bytes[..4], &[b'H', b'M', b'B', 0x02]);

    let back = test_deserialize(&bytes).expect("test: deserialize V2");
    assert_eq!(back.index, block.index);
    assert_eq!(back.hash, block.hash);
}

#[test]
fn test_block_format_v1_still_works() {
    use blockmatrix::blockchain::block::Block;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::persistence::blockchain_storage::{
        test_deserialize, test_serialize_v1,
    };

    let coord = MatrixCoordinate::new(1, 2, 3).expect("test: coord");
    let block = Block::genesis(coord);
    let bytes = test_serialize_v1(&block).expect("test: serialize V1");
    assert_eq!(&bytes[..4], &[b'H', b'M', b'B', 0x01]);
    let back = test_deserialize(&bytes).expect("test: deserialize V1");
    assert_eq!(back.index, block.index);
    assert_eq!(back.hash, block.hash);
}

#[test]
fn test_legacy_raw_bincode_still_works() {
    use blockmatrix::blockchain::block::Block;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::persistence::blockchain_storage::test_deserialize;

    let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
    let block = Block::genesis(coord);
    // Raw bincode without header (legacy path).
    let raw = bincode::serialize(&block).expect("test: bincode");
    let back = test_deserialize(&raw).expect("test: legacy deserialize");
    assert_eq!(back.index, block.index);
    assert_eq!(back.hash, block.hash);
}

// ── 10–11. Protocol-version handling at the IPC server boundary ───────
//
// Build a minimal IpcServer with a ping handler, send requests with
// mismatched/matching `protocol_version`, observe outcomes.

#[tokio::test]
async fn test_protocol_version_mismatch_rejected() {
    use blockmatrix::ipc::handler::RequestHandler;
    use blockmatrix::ipc::protocol::{
        RpcRequest, RpcResponse, IPC_PROTOCOL_VERSION, PROTOCOL_VERSION_MISMATCH,
    };
    use blockmatrix::ipc::server::IpcServer;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let mut h = RequestHandler::new();
    h.register(
        "ping",
        Arc::new(|_| Box::pin(async { Ok(serde_json::json!("pong")) })),
    );
    let h = Arc::new(h);
    let tmp = TempDir::new().expect("test: tempdir");
    let sock = tmp.path().join("j1.sock");
    let server = Arc::new(IpcServer::with_path(h, sock.clone()).expect("test: server"));
    let server_clone = server.clone();
    let task = tokio::spawn(async move {
        let _ = server_clone.run().await;
    });

    // Wait for socket
    for _ in 0..200 {
        if sock.exists() && UnixStream::connect(&sock).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Determine a mismatched major version (different from current).
    let current_major = blockmatrix::ipc::protocol::major_version(IPC_PROTOCOL_VERSION)
        .unwrap_or(0);
    let mismatched_major = current_major + 5; // far away
    let mismatched_version = format!("{}.0.0", mismatched_major);

    let mut req = RpcRequest::new("ping", serde_json::json!(null));
    req.protocol_version = Some(mismatched_version);

    let stream = UnixStream::connect(&sock).await.expect("test: connect");
    let (reader, mut writer) = stream.into_split();
    let mut bytes = serde_json::to_vec(&req).expect("test: serialize");
    bytes.push(b'\n');
    writer.write_all(&bytes).await.expect("test: write");
    let mut br = BufReader::new(reader);
    let mut line = String::new();
    br.read_line(&mut line).await.expect("test: read");
    let resp: RpcResponse = serde_json::from_str(line.trim()).expect("test: parse");
    let err = resp.error.expect("test: should be error");
    assert_eq!(err.code, PROTOCOL_VERSION_MISMATCH);
    assert!(
        err.message.contains("incompatible") || err.message.contains("hypermesh update"),
        "expected helpful upgrade hint, got: {}",
        err.message
    );

    server.shutdown();
    let _ = task.await;
}

#[tokio::test]
async fn test_protocol_version_minor_mismatch_accepted() {
    use blockmatrix::ipc::handler::RequestHandler;
    use blockmatrix::ipc::protocol::{
        RpcRequest, RpcResponse, IPC_PROTOCOL_VERSION,
    };
    use blockmatrix::ipc::server::IpcServer;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let mut h = RequestHandler::new();
    h.register(
        "ping",
        Arc::new(|_| Box::pin(async { Ok(serde_json::json!("pong")) })),
    );
    let h = Arc::new(h);
    let tmp = TempDir::new().expect("test: tempdir");
    let sock = tmp.path().join("j1-minor.sock");
    let server = Arc::new(IpcServer::with_path(h, sock.clone()).expect("test: server"));
    let server_clone = server.clone();
    let task = tokio::spawn(async move {
        let _ = server_clone.run().await;
    });
    for _ in 0..200 {
        if sock.exists() && UnixStream::connect(&sock).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Same major, different minor.
    let current_major = blockmatrix::ipc::protocol::major_version(IPC_PROTOCOL_VERSION)
        .unwrap_or(0);
    let minor_diff_version = format!("{}.99.0", current_major);

    let mut req = RpcRequest::new("ping", serde_json::json!(null));
    req.protocol_version = Some(minor_diff_version);

    let stream = UnixStream::connect(&sock).await.expect("test: connect");
    let (reader, mut writer) = stream.into_split();
    let mut bytes = serde_json::to_vec(&req).expect("test: serialize");
    bytes.push(b'\n');
    writer.write_all(&bytes).await.expect("test: write");
    let mut br = BufReader::new(reader);
    let mut line = String::new();
    br.read_line(&mut line).await.expect("test: read");
    let resp: RpcResponse = serde_json::from_str(line.trim()).expect("test: parse");
    assert!(
        resp.error.is_none(),
        "minor-version mismatch should be accepted; got: {:?}",
        resp.error
    );
    assert_eq!(resp.result.expect("test: result"), serde_json::json!("pong"));

    server.shutdown();
    let _ = task.await;
}
