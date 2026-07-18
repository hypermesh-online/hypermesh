// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Store IPC handler — ingest a file through the asset pipeline, persist
//! shards locally, and distribute to network peers when available.

use std::sync::Arc;

use crate::assets::core::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope, NodeFingerprint,
};
use crate::assets::pipeline::orchestrator::{AssetPipeline, ProcessedAsset};
use crate::assets::pipeline::{Asset, PipelineInputMetadata};
use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::shard_distribution::distribute_to_peers;
use crate::sharing::key_wrap::KeyEnvelope;
use crate::sharing::shard_map::ShardMap;
use hypermesh_lib::{ContentHash, NodeEncryptor, PrivacyMode};
use trustchain::proof_of_state::StateProof;

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
    let stored_shards = store_shards_locally(state, &processed).await;
    let shard_bytes: Vec<[u8; 32]> = stored_shards.iter().map(|(b, _)| *b).collect();
    let shard_hashes: Vec<String> = stored_shards.iter().map(|(_, h)| h.clone()).collect();
    persist_shard_map(state, &processed, &shard_hashes, privacy_mode)
        .map_err(|e| rpc_err(-32004, format!("persist failed: {e}")))?;

    // 5. Register the sharded asset on-chain BEFORE distributing.
    //
    // A6.1: this is the fix. Without an on-chain `StoragePointer::Sharded`
    // entry listing these shard hashes, `NodeBlockchain::authorizes_shard`
    // returns false — which blocks BOTH the publisher's own serve gate
    // (`authorize_shard_fetch`) and a fetcher's re-seed gate
    // (`reannounce_authorized`). Registration failure is a HARD error: an
    // unregistered publish is exactly the bug being fixed. Distribution runs
    // only after registration succeeds (order: persist → register → distribute).
    let registered_block =
        register_sharded_asset_onchain(state, &processed, &shard_bytes, privacy_mode).await?;

    // 6. Distribute to network peers if transport available
    let distribution = distribute_if_possible(state, &processed).await;

    Ok(serde_json::json!({
        "asset_id": asset_id,
        "shard_count": processed.shards.len(),
        "original_size": processed.stats.original_size,
        "final_size": processed.stats.final_size,
        "registered_block": registered_block,
        "shard_hashes": shard_hashes,
        "distributed": distribution,
    }))
}

/// Register the sharded asset on this node's blockchain.
///
/// Writes a single block whose entry carries a `StoragePointer::Sharded` with
/// the asset's shard hashes and their matrix placements, with the node's
/// current `StateProof` bound to the asset's content hash (`new_bound` → the
/// signed-to-content invariant holds by construction, so `add_block`'s
/// per-entry checks pass). This is what makes `authorizes_shard(shard_id)`
/// return true for every shard of this asset.
///
/// Returns the index of the newly-added block.
async fn register_sharded_asset_onchain(
    state: &DaemonState,
    processed: &ProcessedAsset,
    shard_bytes: &[[u8; 32]],
    privacy_mode: PrivacyMode,
) -> Result<u64, RpcError> {
    // asset_hash: the file's BLAKE3 content address (asset_id is its hex form).
    let asset_hash: [u8; 32] = hex::decode(&processed.asset_id)
        .ok()
        .and_then(|v| <[u8; 32]>::try_from(v).ok())
        .ok_or_else(|| {
            rpc_err(
                INTERNAL_ERROR,
                format!("asset_id is not a 32-byte hex hash: {}", processed.asset_id),
            )
        })?;

    // placements: the matrix coordinate of each shard's placement (empty is
    // fine — `authorizes_shard` scans only `shard_hashes`).
    let placements: Vec<MatrixCoordinate> = processed
        .distributed
        .placements
        .iter()
        .map(|p| p.position)
        .collect();

    // Node's current four-proof StateProof (same daemon-path convention the
    // dashboard/domain handlers use before calling `add_block`).
    let state_proof = StateProof::generate_from_network(&state.node_id)
        .await
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("PoS proof generation failed: {e}")))?;

    // Asset registration metadata. Network scope follows the node's
    // PrivacyMode: Bounded (Private) assets register in a node-scoped Private
    // registry; Unbounded (Public/Anonymous) assets register Global.
    let network_scope = if privacy_mode.scope == hypermesh_lib::AccessScope::Bounded {
        node_private_scope(&state.node_id)
    } else {
        NetworkScope::Global
    };
    let asset_data = AssetData {
        config: format!("STORE:SHARDED:{}", processed.asset_id).into_bytes(),
        definition: asset_hash.to_vec(),
        metadata: Vec::new(),
    };
    let registration = AssetRegistration::from_asset_data(
        &asset_data,
        network_scope,
        AssetCategory::BaseSystem(BaseSystemType::Storage),
    );

    // Bind the proof to the asset content hash and add the block via the
    // sanctioned chokepoint. `new_bound` guarantees `content_binding_ok()`.
    let entry = BlockAssetEntry::new_bound(
        asset_hash,
        &state_proof,
        StoragePointer::Sharded {
            shard_hashes: shard_bytes.to_vec(),
            placements,
        },
        registration,
    );
    let block = state
        .blockchain
        .add_block(vec![entry])
        .await
        .map_err(|e| rpc_err(INTERNAL_ERROR, format!("on-chain shard registration failed: {e}")))?;

    Ok(block.index)
}

/// Derive a node-scoped `NetworkScope::Private` from the node's hex id.
///
/// The node id is `BLAKE3(FALCON pubkey)` hex; decode it into the 32-byte
/// `NodeFingerprint`. If it is not 32-byte hex (unexpected), fall back to
/// `Global` — the scope only affects the registration's own content hash,
/// never the shard-authorization gate.
fn node_private_scope(node_id: &str) -> NetworkScope {
    hex::decode(node_id)
        .ok()
        .and_then(|v| <[u8; 32]>::try_from(v).ok())
        .map(|b| NetworkScope::Private(NodeFingerprint(b)))
        .unwrap_or(NetworkScope::Global)
}

/// Store each shard in the local ShardStore.
///
/// Returns each shard's BLAKE3 hash in BOTH forms — the raw `[u8; 32]` bytes
/// (for on-chain `StoragePointer::Sharded` registration, which is what the
/// per-asset shard-authorization gate scans) and the hex string (for the
/// on-disk shard map + IPC response). Returning the bytes here avoids
/// re-decoding the hex at the registration call site.
async fn store_shards_locally(
    state: &DaemonState,
    processed: &ProcessedAsset,
) -> Vec<([u8; 32], String)> {
    let mut hashes = Vec::with_capacity(processed.shards.len());
    for shard in &processed.shards {
        let hash_bytes = *blake3::hash(&shard.data).as_bytes();
        let content_hash = ContentHash(hash_bytes);
        state.shard_store.store(content_hash, shard.data.clone()).await;
        hashes.push((hash_bytes, hex::encode(hash_bytes)));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A6.1 core fix: `handle_store` must register the asset's shards on-chain
    /// so that `NodeBlockchain::authorizes_shard(shard_id)` returns true — the
    /// gate that both the publisher's serve path (`authorize_shard_fetch`) and a
    /// fetcher's re-seed path (`reannounce_authorized`) depend on.
    ///
    /// LIGHTER PATH CHOSEN: drive the full `handle_store` end-to-end against the
    /// shared in-process `DaemonState` fixture (`test-node`, no network → the
    /// distribute step returns `no_network`, which is fine). This exercises the
    /// real publish flow rather than the helper in isolation.
    #[tokio::test]
    async fn handle_store_registers_shards_onchain() {
        let state = crate::ipc::handlers::tests::test_state().await;

        // Write a temp file to ingest.
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let file_path = tmp.path().join("payload.bin");
        // Enough data to exercise real Reed-Solomon sharding.
        let data = b"HYPERMESH-A6.1-STORE-REGISTER-".repeat(500);
        std::fs::write(&file_path, &data).expect("test: write payload");

        let params = serde_json::json!({
            "path": file_path.to_string_lossy(),
        });

        let result = handle_store(params, &state)
            .await
            .expect("test: handle_store must succeed");

        // The response surfaces the new on-chain fields.
        assert!(
            result["registered_block"].as_u64().is_some(),
            "response must carry the registered block index",
        );
        let shard_hashes = result["shard_hashes"]
            .as_array()
            .expect("test: shard_hashes array present");
        assert!(!shard_hashes.is_empty(), "asset must produce at least one shard");

        // Every returned shard hash must now be authorized on-chain — this is
        // the exact gate the serve + re-seed paths call. Assert for all of them.
        for h in shard_hashes {
            let hex = h.as_str().expect("test: shard hash is a hex string");
            let bytes: [u8; 32] = hex::decode(hex)
                .ok()
                .and_then(|v| <[u8; 32]>::try_from(v).ok())
                .expect("test: shard hash is 32-byte hex");
            assert!(
                state.blockchain.authorizes_shard(&bytes).await,
                "shard {hex} must be authorized on-chain after store",
            );
        }

        // Sanity: a shard hash that was NOT stored is NOT authorized.
        assert!(
            !state.blockchain.authorizes_shard(&[0xEEu8; 32]).await,
            "an unrelated shard id must not be authorized",
        );
    }
}
