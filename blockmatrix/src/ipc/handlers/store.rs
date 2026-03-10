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
use hypermesh_lib::ContentHash;

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
    let processed = pipeline
        .process_asset(asset)
        .await
        .map_err(|e| rpc_err(-32003, format!("pipeline processing failed: {e}")))?;

    // 4. Store shards locally + persist to disk
    let shard_hashes = store_shards_locally(state, &processed).await;
    persist_shard_map(state, &processed, &shard_hashes)
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
fn persist_shard_map(
    state: &DaemonState,
    processed: &ProcessedAsset,
    shard_hashes: &[String],
) -> Result<(), String> {
    let maps_dir = state.data_dir.join("shard_maps");
    std::fs::create_dir_all(&maps_dir)
        .map_err(|e| format!("create shard_maps dir: {e}"))?;

    let map = serde_json::json!({
        "asset_id": processed.asset_id,
        "shard_hashes": shard_hashes,
        "decryption_key": processed.decryption_key,
        "shard_count": processed.shards.len(),
        "original_size": processed.stats.original_size,
        "shard_metadata": processed.shards.iter().map(|s| &s.metadata)
            .collect::<Vec<_>>(),
    });

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
