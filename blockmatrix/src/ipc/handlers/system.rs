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
//!   version. When an entry exists the handler:
//!     1. Re-verifies the release's FALCON-1024 signature against the
//!        subscriber's configured foundation public key.
//!     2. Checks protocol-version compatibility (major must match the
//!        running daemon — reuses P6's
//!        [`crate::ipc::protocol::protocol_versions_compatible`]).
//!     3. Gates on `requires_min_version` (refuses a jump that skips a
//!        required compatibility step).
//!     4. Stages the binary to a temp path, verifies its SHA-256 against
//!        the signed `binary_hashes`, and atomically renames it over the
//!        target, preserving all DURABLE state.
//!     5. Schedules a restart (the daemon handles SIGTERM gracefully).
//!   On signature failure or version-incompatibility it returns an error
//!   and `applied` never becomes `true`.
//!
//! See `papers/HYPERMESH.md` Phase J for the upgrade-substrate
//! commitment.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{
    protocol_versions_compatible, RpcError, INTERNAL_ERROR, INVALID_PARAMS,
    IPC_PROTOCOL_VERSION, PROTOCOL_VERSION_MISMATCH,
};
use crate::ipc::state::DaemonState;
use crate::release_feed::{
    compare_versions, ReleaseChannel, ReleaseFeedEntry, ReleaseFeedSubscriber,
};

const DEFAULT_TARGET_TRIPLE: &str = "x86_64-unknown-linux-musl";

/// Outcome of staging + swapping a verified release binary.
///
/// Separated from the RPC handler so the verify+stage+swap logic is unit-testable
/// without a live IPC round-trip. `restart_scheduled` is `false` in tests (no
/// candidate binary), `true` in production once the atomic rename succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwapOutcome {
    /// Absolute path the new binary was installed to.
    pub installed_to: PathBuf,
    /// Whether a restart was scheduled after a successful swap.
    pub restart_scheduled: bool,
}

/// Errors from the verify → version-gate → stage → swap pipeline.
#[derive(Debug)]
pub(crate) enum ApplyUpdateError {
    /// The release's FALCON signature did not verify against the foundation key.
    SignatureInvalid,
    /// The daemon has no foundation public key configured (cannot verify).
    NoFoundationKey,
    /// The release's major protocol version is incompatible with the daemon.
    VersionIncompatible { release: String, daemon: String },
    /// `requires_min_version` is newer than the running daemon — the jump would
    /// skip a required compatibility step.
    MinVersionUnmet { requires: String, running: String },
    /// The staged binary's SHA-256 did not match the signed hash.
    HashMismatch { expected: String, actual: String },
    /// A filesystem operation (download/stage/rename) failed.
    Io(String),
}

/// Verify a release entry against the foundation key, then gate it on protocol
/// compatibility and `requires_min_version`. Pure and testable: performs NO
/// filesystem I/O.
pub(crate) fn verify_and_gate(
    entry: &ReleaseFeedEntry,
    foundation_pubkey: Option<&[u8]>,
    running_version: &str,
) -> Result<(), ApplyUpdateError> {
    // 1. Signature must verify against the configured foundation key.
    let pubkey = foundation_pubkey.ok_or(ApplyUpdateError::NoFoundationKey)?;
    entry
        .verify(pubkey)
        .map_err(|_| ApplyUpdateError::SignatureInvalid)?;

    // 2. Major protocol version must match the running daemon (P6 rule).
    if !protocol_versions_compatible(&entry.version, running_version) {
        return Err(ApplyUpdateError::VersionIncompatible {
            release: entry.version.clone(),
            daemon: running_version.to_string(),
        });
    }

    // 3. requires_min_version must not be newer than the running daemon — a jump
    //    that skips a required compatibility step is refused.
    if let Some(ref min) = entry.requires_min_version {
        if compare_versions(running_version, min) == std::cmp::Ordering::Less {
            return Err(ApplyUpdateError::MinVersionUnmet {
                requires: min.clone(),
                running: running_version.to_string(),
            });
        }
    }

    Ok(())
}

/// Stage a candidate binary and atomically swap it over `target`, verifying its
/// SHA-256 against `expected_hash` first. The caller must have already run
/// [`verify_and_gate`].
///
/// Safety: the new binary is written to a sibling temp path, fsync'd, hash-checked,
/// then `rename`d over the target (atomic on the same filesystem). Durable state
/// (blockchain, identity DER, matrix snapshots, WAL) lives elsewhere and is never
/// touched. `schedule_restart` is invoked only after a successful rename.
pub(crate) fn stage_and_swap(
    candidate_bytes: &[u8],
    expected_hash: &str,
    target: &Path,
    schedule_restart: impl FnOnce(),
) -> Result<SwapOutcome, ApplyUpdateError> {
    // Hash the candidate before it touches the target path.
    let actual_hash = sha256_hex(candidate_bytes);
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        return Err(ApplyUpdateError::HashMismatch {
            expected: expected_hash.to_string(),
            actual: actual_hash,
        });
    }

    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let staging = dir.join(format!(
        ".hypermesh-upgrade-staging-{}",
        std::process::id()
    ));

    // Write → fsync → set executable → atomic rename over the target.
    write_and_sync(&staging, candidate_bytes)
        .map_err(|e| ApplyUpdateError::Io(format!("stage write failed: {e}")))?;
    make_executable(&staging)
        .map_err(|e| ApplyUpdateError::Io(format!("chmod +x staging failed: {e}")))?;
    std::fs::rename(&staging, target).map_err(|e| {
        // Best-effort cleanup of the staging file on rename failure.
        let _ = std::fs::remove_file(&staging);
        ApplyUpdateError::Io(format!("atomic rename over {} failed: {e}", target.display()))
    })?;

    schedule_restart();
    Ok(SwapOutcome {
        installed_to: target.to_path_buf(),
        restart_scheduled: true,
    })
}

/// SHA-256 hex digest (matches the `binary_hashes` encoding in the release feed).
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Write `bytes` to `path`, flushing and fsync'ing before returning.
fn write_and_sync(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    f.flush()?;
    f.sync_all()?;
    Ok(())
}

/// Mark `path` executable (owner rwx, group/other rx) on Unix; no-op elsewhere.
fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

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
            Box::pin(async move { apply_update_impl(&s, params).await })
        }),
    );
}

/// Map an [`ApplyUpdateError`] to a JSON-RPC error. Signature failure and
/// version-incompatibility are `INVALID_PARAMS`/`PROTOCOL_VERSION_MISMATCH` so a
/// caller can distinguish "you asked for the wrong thing" from an internal fault.
fn apply_error_to_rpc(e: ApplyUpdateError) -> RpcError {
    match e {
        ApplyUpdateError::SignatureInvalid => RpcError {
            code: INVALID_PARAMS,
            message: "release signature failed FALCON-1024 verification; refusing to apply"
                .into(),
            data: None,
        },
        ApplyUpdateError::NoFoundationKey => RpcError {
            code: INTERNAL_ERROR,
            message: "no foundation public key configured; cannot verify release".into(),
            data: None,
        },
        ApplyUpdateError::VersionIncompatible { release, daemon } => RpcError {
            code: PROTOCOL_VERSION_MISMATCH,
            message: format!(
                "release {release} is protocol-incompatible with daemon {daemon} \
                 (major version differs); refusing to apply"
            ),
            data: None,
        },
        ApplyUpdateError::MinVersionUnmet { requires, running } => RpcError {
            code: INVALID_PARAMS,
            message: format!(
                "release requires_min_version {requires} is newer than running {running}; \
                 upgrade to the required compatibility step first"
            ),
            data: None,
        },
        ApplyUpdateError::HashMismatch { expected, actual } => RpcError {
            code: INVALID_PARAMS,
            message: format!(
                "staged binary hash {actual} does not match signed hash {expected}; \
                 refusing to swap"
            ),
            data: None,
        },
        ApplyUpdateError::Io(msg) => RpcError {
            code: INTERNAL_ERROR,
            message: format!("binary swap failed: {msg}"),
            data: None,
        },
    }
}

/// Core of `system.apply_update`. Split out so both the handler closure and the
/// tests can call it directly.
async fn apply_update_impl(
    s: &Arc<DaemonState>,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
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

    // 1-3. Re-verify signature + gate on protocol compatibility and
    // requires_min_version. On any failure, `applied` never becomes true.
    let foundation_pubkey = subscriber.foundation_pubkey().await;
    verify_and_gate(&entry, foundation_pubkey.as_deref(), IPC_PROTOCOL_VERSION)
        .map_err(apply_error_to_rpc)?;

    // Resolve the target triple's signed binary hash.
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

    // 4-5. Stage + atomic-swap + schedule restart. The candidate binary bytes
    // arrive via the optional base64 `binary` param (a fetched-and-staged blob).
    // Absent that, the verification + gating succeeded but there is nothing to
    // swap yet: report `staged=false` while still confirming the update is
    // authorized. When present, we verify the hash, atomically replace the
    // target, and schedule a graceful restart — `applied` becomes true.
    let target_path: PathBuf = params
        .get("install_path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/local/bin/hypermesh"));

    let candidate_b64 = params.get("binary").and_then(|v| v.as_str());

    match candidate_b64 {
        Some(b64) => {
            use base64::Engine;
            let candidate = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| RpcError {
                    code: INVALID_PARAMS,
                    message: format!("'binary' is not valid base64: {e}"),
                    data: None,
                })?;

            let shutdown_tx = s.shutdown_tx.clone();
            let outcome = stage_and_swap(
                &candidate,
                &expected_hash,
                &target_path,
                move || {
                    // Signal graceful shutdown; the process supervisor re-execs
                    // the freshly-swapped binary. Durable state is untouched.
                    let _ = shutdown_tx.send(true);
                },
            )
            .map_err(apply_error_to_rpc)?;

            Ok(serde_json::json!({
                "status": "applied",
                "applied": true,
                "version": entry.version,
                "channel": channel.as_str(),
                "target": target_triple,
                "installed_to": outcome.installed_to.display().to_string(),
                "restart_scheduled": outcome.restart_scheduled,
                "release_notes_url": entry.release_notes_url,
                "expected_binary_hash": expected_hash,
                "breaking_changes": entry.breaking_changes,
            }))
        }
        None => Ok(serde_json::json!({
            "status": "authorized",
            "applied": false,
            "staged": false,
            "version": entry.version,
            "channel": channel.as_str(),
            "target": target_triple,
            "install_path": target_path.display().to_string(),
            "release_notes_url": entry.release_notes_url,
            "expected_binary_hash": expected_hash,
            "breaking_changes": entry.breaking_changes,
            "note": "signature + version compatibility verified; supply base64 'binary' \
                     to stage and atomically swap",
        })),
    }
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
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
            capability_token_issuer: None,
            revocation_registry: Arc::new(crate::auth::RevocationRegistry::new()),
            light_sync_manager: None,
            catalog_registry: None,
            inbox_store: None,
        })
    }

    /// SHA-256 hex of the canonical test binary blob, so `binary_hashes` matches
    /// what `stage_and_swap` computes over the same bytes.
    fn test_binary_bytes() -> Vec<u8> {
        b"#!/bin/sh\n# hypermesh test binary\nexit 0\n".to_vec()
    }

    fn test_binary_sha256() -> String {
        super::sha256_hex(&test_binary_bytes())
    }

    fn signed_entry(
        signer: &FalconIdentity,
        version: &str,
        channel: ReleaseChannel,
    ) -> ReleaseFeedEntry {
        signed_entry_with(signer, version, channel, &test_binary_sha256(), None)
    }

    fn signed_entry_with(
        signer: &FalconIdentity,
        version: &str,
        channel: ReleaseChannel,
        binary_hash: &str,
        requires_min_version: Option<&str>,
    ) -> ReleaseFeedEntry {
        let mut hashes = HashMap::new();
        hashes.insert(
            "x86_64-unknown-linux-musl".to_string(),
            binary_hash.to_string(),
        );
        let mut entry = ReleaseFeedEntry {
            version: version.to_string(),
            channel,
            binary_hashes: hashes,
            release_notes_url: format!("https://release.hypermesh.online/{}", version),
            signed_by: signer.public_key.clone(),
            signature: Vec::new(),
            requires_min_version: requires_min_version.map(|s| s.to_string()),
            breaking_changes: false,
            issued_at: SystemTime::now(),
        };
        entry.signature = signer
            .sign(&entry.signing_payload())
            .expect("test: sign");
        entry
    }

    /// The daemon's running protocol version = its `CARGO_PKG_VERSION`. Tests use
    /// a release version with the SAME major so version-compat passes; a distinct
    /// minor keeps `check_update` "newer" logic meaningful.
    fn compat_version() -> String {
        let major = IPC_PROTOCOL_VERSION
            .split('.')
            .next()
            .unwrap_or("1");
        format!("{major}.999.0")
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
        let newer = compat_version();
        let entry = signed_entry(&foundation, &newer, ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");
        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new("system.check_update", serde_json::json!({}));
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["up_to_date"], false);
        assert_eq!(result["available_version"], newer);
    }

    #[tokio::test]
    async fn apply_update_rejects_when_not_configured() {
        let state = make_state(None).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new(
            "system.apply_update",
            serde_json::json!({"version": compat_version()}),
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
    async fn apply_update_authorizes_when_entry_present_without_binary() {
        let foundation = FalconIdentity::generate();
        let sub = Arc::new(ReleaseFeedSubscriber::with_foundation_pubkey(
            foundation.public_key.clone(),
        ));
        let version = compat_version();
        let entry = signed_entry(&foundation, &version, ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");
        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new(
            "system.apply_update",
            serde_json::json!({"version": version}),
        );
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        // No binary supplied: verified + authorized but not yet applied.
        assert_eq!(result["status"], "authorized");
        assert_eq!(result["applied"], false);
        assert_eq!(result["version"], version);
    }

    #[tokio::test]
    async fn apply_update_stages_and_applies_with_valid_binary() {
        let foundation = FalconIdentity::generate();
        let sub = Arc::new(ReleaseFeedSubscriber::with_foundation_pubkey(
            foundation.public_key.clone(),
        ));
        let version = compat_version();
        let entry = signed_entry(&foundation, &version, ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");
        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);

        // Stage into a temp dir so the atomic rename has a real target.
        let dir = tempfile::tempdir().expect("test: tempdir");
        let target = dir.path().join("hypermesh");
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(test_binary_bytes());

        let req = RpcRequest::new(
            "system.apply_update",
            serde_json::json!({
                "version": version,
                "binary": b64,
                "install_path": target.display().to_string(),
            }),
        );
        let resp = h.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "applied");
        assert_eq!(result["applied"], true);
        assert_eq!(result["restart_scheduled"], true);
        // The swap actually placed the binary at the target path.
        assert!(target.exists(), "swapped binary should exist at target");
    }

    #[tokio::test]
    async fn apply_update_rejects_binary_with_wrong_hash() {
        let foundation = FalconIdentity::generate();
        let sub = Arc::new(ReleaseFeedSubscriber::with_foundation_pubkey(
            foundation.public_key.clone(),
        ));
        let version = compat_version();
        let entry = signed_entry(&foundation, &version, ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");
        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);

        let dir = tempfile::tempdir().expect("test: tempdir");
        let target = dir.path().join("hypermesh");
        use base64::Engine;
        // Wrong bytes → hash mismatch against the signed hash.
        let b64 = base64::engine::general_purpose::STANDARD.encode(b"tampered binary");
        let req = RpcRequest::new(
            "system.apply_update",
            serde_json::json!({
                "version": version,
                "binary": b64,
                "install_path": target.display().to_string(),
            }),
        );
        let resp = h.dispatch(req).await;
        let err = resp.error.expect("test: expected hash-mismatch error");
        assert!(err.message.contains("does not match"), "got: {}", err.message);
        assert!(!target.exists(), "target must not be swapped on hash mismatch");
    }

    #[tokio::test]
    async fn apply_update_rejects_version_incompatible_release() {
        let foundation = FalconIdentity::generate();
        let sub = Arc::new(ReleaseFeedSubscriber::with_foundation_pubkey(
            foundation.public_key.clone(),
        ));
        // Bump the major so it is protocol-incompatible with the daemon.
        let daemon_major: u64 = IPC_PROTOCOL_VERSION
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let incompatible = format!("{}.0.0", daemon_major + 1);
        let entry = signed_entry(&foundation, &incompatible, ReleaseChannel::Stable);
        sub.ingest(entry).await.expect("test: ingest");
        let state = make_state(Some(sub)).await;
        let mut h = RequestHandler::new();
        register(&mut h, &state);
        let req = RpcRequest::new(
            "system.apply_update",
            serde_json::json!({"version": incompatible}),
        );
        let resp = h.dispatch(req).await;
        let err = resp.error.expect("test: expected version-mismatch error");
        assert_eq!(err.code, PROTOCOL_VERSION_MISMATCH);
    }

    #[test]
    fn verify_and_gate_rejects_bad_signature() {
        let foundation = FalconIdentity::generate();
        let attacker = FalconIdentity::generate();
        let version = compat_version();
        let entry = signed_entry(&foundation, &version, ReleaseChannel::Stable);
        // Verified against the WRONG key → signature invalid.
        let r = verify_and_gate(&entry, Some(&attacker.public_key), &version);
        assert!(matches!(r, Err(ApplyUpdateError::SignatureInvalid)));
    }

    #[test]
    fn verify_and_gate_rejects_unmet_min_version() {
        let foundation = FalconIdentity::generate();
        let daemon_major: u64 = IPC_PROTOCOL_VERSION
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let running = format!("{daemon_major}.1.0");
        let release = format!("{daemon_major}.9.0");
        // requires_min_version newer than running → refused.
        let min_required = format!("{daemon_major}.5.0");
        let entry = signed_entry_with(
            &foundation,
            &release,
            ReleaseChannel::Stable,
            &test_binary_sha256(),
            Some(&min_required),
        );
        let r = verify_and_gate(&entry, Some(&foundation.public_key), &running);
        assert!(matches!(r, Err(ApplyUpdateError::MinVersionUnmet { .. })));
    }

    #[test]
    fn stage_and_swap_places_binary_and_schedules_restart() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let target = dir.path().join("hypermesh");
        let bytes = test_binary_bytes();
        let mut restarted = false;
        let outcome = stage_and_swap(&bytes, &test_binary_sha256(), &target, || {
            restarted = true;
        })
        .expect("test: swap succeeds");
        assert!(outcome.restart_scheduled);
        assert!(restarted);
        assert!(target.exists());
        let installed = std::fs::read(&target).expect("test: read installed");
        assert_eq!(installed, bytes);
    }
}
