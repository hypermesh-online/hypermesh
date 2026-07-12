// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Store IPC handler — ingest a file through the asset pipeline, persist
//! shards locally, and distribute to network peers when available.

use std::sync::Arc;

use crate::assets::pipeline::orchestrator::{AssetPipeline, ProcessedAsset};
use crate::assets::pipeline::{Asset, PipelineInputMetadata};
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use crate::network::shard_distribution::distribute_to_peers;
use crate::sharing::key_wrap::KeyEnvelope;
use crate::sharing::shard_map::ShardMap;
use hypermesh_lib::{ContentHash, NodeEncryptor, PrivacyMode};

/// Register the `store` IPC method.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    let s = state.clone();
    handler.register(
        "store",
        Arc::new(move |params| {
            let s = s.clone();
            Box::pin(async move { handle_store(params, &s).await })
        }),
    );
}

fn rpc_err(code: i64, message: impl Into<String>) -> RpcError {
    RpcError {
        code,
        message: message.into(),
        data: None,
    }
}

/// Interpret the daemon's stored privacy-mode string.
///
/// `DaemonState.privacy_mode` is `format!("{:?}", PrivacyMode)` (a Debug
/// string) or a Display name. Any `Unbounded` scope (Public/Anonymous) maps to
/// a cleartext (unencrypted) pipeline; everything else defaults to Private
/// (encrypted). Defaulting to Private is the safe choice.
fn privacy_mode_from_display(s: &str) -> PrivacyMode {
    if s.contains("Unbounded") || s == "Public" || s == "Anonymous" {
        // Public and Anonymous are both Unbounded scope → cleartext shards.
        PrivacyMode::PUBLIC
    } else {
        PrivacyMode::PRIVATE
    }
}

async fn handle_store(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let path_str = params["path"]
        .as_str()
        .ok_or_else(|| rpc_err(INVALID_PARAMS, "missing 'path' parameter"))?;

    let path = std::path::Path::new(path_str);

    // 1. Read file
    let file_data = std::fs::read(path)
        .map_err(|e| rpc_err(-32001, format!("failed to read {path_str}: {e}")))?;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());
    let file_size = file_data.len();

    // 2. Content-address the asset
    let asset_id = hex::encode(blake3::hash(&file_data).as_bytes());

    // 3. Run the pipeline
    let asset = Asset {
        id: asset_id.clone(),
        data: file_data,
        metadata: PipelineInputMetadata {
            name: file_name,
            content_type: "application/octet-stream".to_string(),
            size: file_size,
            created_at: chrono::Utc::now().timestamp(),
            custom: std::collections::HashMap::new(),
        },
    };

    let pipeline = AssetPipeline::default()
        .map_err(|e| rpc_err(-32002, format!("pipeline init failed: {e}")))?;

    // Honor the node's PrivacyMode for the encryption stage: Private assets are
    // Kyber-encrypted (key custodied + wrapped per-recipient); Public/Anonymous
    // assets are content-addressed cleartext shards (no key to custody).
    let privacy_mode = privacy_mode_from_display(&state.privacy_mode);
    let processed = pipeline
        .process_asset_with_privacy(asset, privacy_mode)
        .await
        .map_err(|e| rpc_err(-32003, format!("pipeline processing failed: {e}")))?;

    // 4. Store shards locally + persist to disk
    let shard_hashes = store_shards_locally(state, &processed).await;
    persist_shard_map(state, &processed, &shard_hashes, privacy_mode)
        .map_err(|e| rpc_err(-32004, format!("persist failed: {e}")))?;

    // 5. Distribute to network peers if transport available
    let distribution = distribute_if_possible(state, &processed).await;

    Ok(serde_json::json!({
        "asset_id": asset_id,
        "shard_count": processed.shards.len(),
        "original_size": processed.stats.original_size,
        "final_size": processed.stats.final_size,
        "distributed": distribution,
    }))
}

/// Store each shard in the local ShardStore. Returns BLAKE3 hashes.
async fn store_shards_locally(
    state: &DaemonState,
    processed: &ProcessedAsset,
) -> Vec<String> {
    let mut hashes = Vec::with_capacity(processed.shards.len());
    for shard in &processed.shards {
        let hash_bytes = *blake3::hash(&shard.data).as_bytes();
        let content_hash = ContentHash(hash_bytes);
        state.shard_store.store(content_hash, shard.data.clone()).await;
        hashes.push(hex::encode(hash_bytes));
    }
    hashes
}

/// Persist the shard map and raw shard data to disk.
///
/// The on-disk map carries locate + integrity data plus — for encrypted
/// (Private) assets — a [`KeyEnvelope`] wrapped for the OWNER's own Kyber
/// identity (self-custody). The raw Kyber secret key is never serialized: the
/// owner recovers it on fetch by decapsulating with the node Kyber secret held
/// in the keystore. Public/Anonymous assets carry no envelope (cleartext).
fn persist_shard_map(
    state: &DaemonState,
    processed: &ProcessedAsset,
    shard_hashes: &[String],
    privacy_mode: PrivacyMode,
) -> Result<(), String> {
    let maps_dir = state.data_dir.join("shard_maps");
    std::fs::create_dir_all(&maps_dir)
        .map_err(|e| format!("create shard_maps dir: {e}"))?;

    // Wrap the decryption key for self-custody when the asset is encrypted.
    // The identity is loaded from the node's on-disk keystore (never held in
    // `DaemonState`) so the raw Kyber secret stays out of shared state.
    let key_envelope = if privacy_mode.scope == hypermesh_lib::AccessScope::Bounded {
        let identity_dir = state.data_dir.join(&state.node_id).join("identity");
        let identity = crate::identity::FalconIdentity::load_or_create(&identity_dir)
            .map_err(|e| format!("load node identity for key self-custody: {e}"))?;
        let env = KeyEnvelope::wrap_for(
            &processed.decryption_key,
            identity.encryption_public_key(),
        )
        .map_err(|e| format!("wrap key for self-custody: {e}"))?;
        Some(env)
    } else {
        None
    };

    let map = ShardMap {
        asset_id: processed.asset_id.clone(),
        shard_hashes: shard_hashes.to_vec(),
        key_envelope,
        shard_count: processed.shards.len(),
        original_size: processed.stats.original_size,
        shard_metadata: processed.shards.iter().map(|s| s.metadata.clone()).collect(),
    };

    let map_path = maps_dir.join(format!("{}.json", processed.asset_id));
    let map_json = serde_json::to_string_pretty(&map)
        .map_err(|e| format!("serialize shard map: {e}"))?;
    std::fs::write(&map_path, &map_json)
        .map_err(|e| format!("write {}: {e}", map_path.display()))?;

    // Persist raw shard data files
    let shards_dir = maps_dir.join(&processed.asset_id);
    std::fs::create_dir_all(&shards_dir)
        .map_err(|e| format!("create shards dir: {e}"))?;

    for (hash_hex, shard) in shard_hashes.iter().zip(processed.shards.iter()) {
        let shard_path = shards_dir.join(hash_hex);
        std::fs::write(&shard_path, &shard.data)
            .map_err(|e| format!("write shard: {e}"))?;
    }

    Ok(())
}

/// Distribute shards to network peers if shard transport is available.
async fn distribute_if_possible(
    state: &DaemonState,
    processed: &ProcessedAsset,
) -> serde_json::Value {
    let (network, transport) = match (&state.network, &state.shard_transport) {
        (Some(net), Some(tr)) => (net, tr),
        _ => {
            return serde_json::json!({
                "sent": 0,
                "kept_local": processed.shards.len(),
                "reason": "no_network",
            });
        }
    };

    let shard_pairs: Vec<(ContentHash, Vec<u8>)> = processed
        .shards
        .iter()
        .map(|s| {
            let hash_bytes = *blake3::hash(&s.data).as_bytes();
            (ContentHash(hash_bytes), s.data.clone())
        })
        .collect();

    let peers = network.get_connected_nodes().await;
    let result = distribute_to_peers(
        &shard_pairs,
        &processed.distributed.placements,
        &peers,
        transport.as_ref(),
    )
    .await;

    serde_json::json!({
        "sent": result.sent,
        "kept_local": result.kept_local,
        "failed": result.failed,
    })
}
