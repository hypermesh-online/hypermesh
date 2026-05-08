// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Phase J.1 — System update IPC handlers.
//!
//! Surfaces foundation-published release feed entries via two methods:
//!
//! - `system.check_update` — reads from
//!   [`ReleaseFeedSubscriber::latest_for_channel`] and compares against
//!   the running daemon's `CARGO_PKG_VERSION`. Returns `up_to_date` if
//!   no entry or the cached version equals the running version, or
//!   `available_version` with release-notes URL otherwise.
//! - `system.apply_update` — orchestrates a binary swap workflow.
//!   Alpha-default: returns `not configured` unless the subscriber is
//!   wired AND a candidate entry actually exists for the requested
//!   version. The actual binary swap (download → verify hash → replace
//!   `/usr/local/bin/hypermesh` → restart daemon) is deferred to a
//!   follow-up sub-step; this handler validates intent and returns the
//!   plan as JSON so an operator can audit.
//!
//! See `papers/HYPERMESH.md` Phase J for the upgrade-substrate
//! commitment.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{
    RpcError, INTERNAL_ERROR, INVALID_PARAMS, IPC_PROTOCOL_VERSION,
};
use crate::ipc::state::DaemonState;
use crate::release_feed::{ReleaseChannel, ReleaseFeedSubscriber};

const DEFAULT_TARGET_TRIPLE: &str = "x86_64-unknown-linux-musl";

/// Parse the optional `channel` parameter; defaults to `stable`.
fn parse_channel(params: &serde_json::Value) -> Result<ReleaseChannel, RpcError> {
    match params.get("channel").and_then(|v| v.as_str()) {
        None => Ok(ReleaseChannel::Stable),
        Some(s) => ReleaseChannel::parse(s).ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: format!("invalid channel '{}': expected stable|beta|nightly", s),
            data: None,
        }),
    }
}

/// Register `system.check_update` and `system.apply_update`.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    register_check_update(handler, state);
    register_apply_update(handler, state);
}

fn register_check_update(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    let s = state.clone();
    handler.register(
        "system.check_update",
        Arc::new(move |params| {
            let s = s.clone();
            Box::pin(async move {
                let channel = parse_channel(&params)?;
                let current_version = IPC_PROTOCOL_VERSION.to_string();

                let subscriber: Arc<ReleaseFeedSubscriber> = match s
                    .release_feed_subscriber
                    .clone()
                {
                    Some(sub) => sub,
                    None => {
                        return Ok(serde_json::json!({
                            "up_to_date": true,
                            "current_version": current_version,
                            "channel": channel.as_str(),
                            "note": "release feed not configured (alpha-default inert)",
                        }));
                    }
                };

                let latest = subscriber.latest_for_channel(channel).await;
                match latest {
                    None => Ok(serde_json::json!({
                        "up_to_date": true,
                        "current_version": current_version,
                        "channel": channel.as_str(),
                    })),
                    Some(entry) if entry.version == current_version => Ok(serde_json::json!({
                        "up_to_date": true,
                        "current_version": current_version,
                        "channel": channel.as_str(),
                    })),
                    Some(entry) => Ok(serde_json::json!({
                        "up_to_date": false,
                        "available_version": entry.version,
                        "current_version": current_version,
                        "channel": channel.as_str(),
                        "release_notes_url": entry.release_notes_url,
                        "breaking_changes": entry.breaking_changes,
                        "requires_min_version": entry.requires_min_version,
                    })),
                }
            })
        }),
    );
}

fn register_apply_update(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    let s = state.clone();
    handler.register(
        "system.apply_update",
        Arc::new(move |params| {
            let s = s.clone();
            Box::pin(async move {
                let target_version = params
                    .get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'version' parameter".into(),
                        data: None,
                    })?
                    .to_string();
                let channel = parse_channel(&params)?;

                let subscriber = s.release_feed_subscriber.clone().ok_or_else(|| RpcError {
                    code: INTERNAL_ERROR,
                    message: "release feed not configured (alpha-default inert); set \
                              release_feed_subscriber on daemon startup to opt-in"
                        .into(),
                    data: None,
                })?;

                // Find an entry matching channel+version.
                let entries = subscriber.all_versions().await;
                let entry = entries
                    .into_iter()
                    .find(|e| e.channel == channel && e.version == target_version)
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: format!(
                            "no cached release entry for version '{}' on channel '{}'",
                            target_version,
                            channel.as_str()
                        ),
                        data: None,
                    })?;

                // Resolve the target triple's binary hash (defaulting
                // to x86_64-unknown-linux-musl).  Phase J.1 alpha
                // returns the verification plan as JSON so an operator
                // can audit; the actual binary swap is deferred to a
                // follow-up so we don't bake permission/elevation
                // logic into the daemon ahead of an explicit opt-in.
                let target_triple = params
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or(DEFAULT_TARGET_TRIPLE)
                    .to_string();
                let expected_hash = entry
                    .binary_hashes
                    .get(&target_triple)
                    .cloned()
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: format!(
                            "release entry for {} has no binary hash for target '{}'",
                            target_version, target_triple
                        ),
                        data: None,
                    })?;

                Ok(serde_json::json!({
                    "status": "validated",
                    "applied": false,
                    "version": entry.version,
                    "channel": channel.as_str(),
                    "target": target_triple,
                    "release_notes_url": entry.release_notes_url,
                    "expected_binary_hash": expected_hash,
                    "breaking_changes": entry.breaking_changes,
                    "note": "binary swap deferred to follow-up sub-step; \
                             current handler validates the upgrade plan only",
                }))
            })
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::node_chain::NodeBlockchain;
    use crate::bootstrap::DnsResolver;
    use crate::ipc::protocol::RpcRequest;
    use crate::ipc::state::DaemonState;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::network::shard_store::ShardStore;
    use crate::persistence::{PersistenceConfig, PersistenceManager};
    use crate::release_feed::{ReleaseChannel, ReleaseFeedEntry, ReleaseFeedSubscriber};
    use hypermesh_lib::NodeSigner;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{Instant, SystemTime};
    use trustchain::FalconIdentity;

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
            PersistenceManager::new(config, "system-test".into())
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
            node_id: "system-test".into(),
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
            release_feed_subscriber: subscriber,
            receipt_validator: Arc::new(
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
        })
    }

    fn signed_entry(
        signer: &FalconIdentity,
        version: &str,
        channel: ReleaseChannel,
    ) -> ReleaseFeedEntry {
        let mut hashes = HashMap::new();
        hashes.insert(
            "x86_64-unknown-linux-musl".to_string(),
            "00".repeat(32),
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
            .expect("test: sign");
        entry
    }

    #[tokio::test]
    async fn check_update_returns_up_to_date_when_no_subscriber() {
        let state = make_state(None).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new("system.check_update", serde_json::json!({}));
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["up_to_date"], true);
    }

    #[tokio::test]
    async fn check_update_returns_available_when_newer_entry() {
        let foundation = FalconIdentity::generate();
        let sub = Arc::new(ReleaseFeedSubscriber::with_foundation_pubkey(
            foundation.public_key.clone(),
        ));
        let entry = signed_entry(&foundation, "99.0.0", ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");
        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new("system.check_update", serde_json::json!({}));
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["up_to_date"], false);
        assert_eq!(result["available_version"], "99.0.0");
    }

    #[tokio::test]
    async fn apply_update_rejects_when_not_configured() {
        let state = make_state(None).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new(
            "system.apply_update",
            serde_json::json!({"version": "0.99.0"}),
        );
        let resp = h.dispatch(req).await;
        let err = resp.error.expect("test: error");
        assert!(
            err.message.contains("not configured"),
            "expected 'not configured', got: {}",
            err.message
        );
    }

    #[tokio::test]
    async fn apply_update_validates_when_entry_present() {
        let foundation = FalconIdentity::generate();
        let sub = Arc::new(ReleaseFeedSubscriber::with_foundation_pubkey(
            foundation.public_key.clone(),
        ));
        let entry = signed_entry(&foundation, "0.99.0", ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");
        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new(
            "system.apply_update",
            serde_json::json!({"version": "0.99.0"}),
        );
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "validated");
        assert_eq!(result["applied"], false);
        assert_eq!(result["version"], "0.99.0");
    }
}
