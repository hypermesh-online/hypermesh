// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Store and Fetch commands for the asset pipeline.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use blockmatrix::assets::pipeline::{
    Asset, AssetPipeline, DecryptionKey, PipelineInputMetadata, ProcessedAsset, Shard,
    ShardMetadata,
};
use blockmatrix::assets::pipeline::distribution::{DistributedAsset, DistributionMetadata};
use blockmatrix::assets::pipeline::PipelineStats;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::NetworkManager;
use blockmatrix::ipc;
use hypermesh_lib::ContentHash;

/// Optional network context for shard distribution during asset storage.
pub struct ShardDistributionCtx {
    pub network: std::sync::Arc<NetworkManager>,
    pub shard_transport: std::sync::Arc<StoqShardTransport>,
}

/// Shard map persisted to disk after Store, used by Fetch to reconstruct.
#[derive(Serialize, Deserialize)]
struct ShardMap {
    asset_id: String,
    shard_hashes: Vec<String>,
    decryption_key: DecryptionKey,
    shard_count: usize,
    original_size: usize,
    shard_metadata: Vec<ShardMetadata>,
}

/// Return the directory used for shard map files (`~/.hypermesh/shard_maps/`).
fn shard_maps_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".hypermesh").join("shard_maps"))
}

/// Run the Store subcommand: ingest a file through the asset pipeline and
/// persist the resulting shards + shard map locally.
pub async fn run_store(
    path: std::path::PathBuf,
    dist_ctx: Option<&ShardDistributionCtx>,
) -> Result<()> {
    let file_data =
        std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());
    let file_size = file_data.len();

    let asset_id = hex::encode(blake3::hash(&file_data).as_bytes());

    info!(
        "Storing file {} ({} bytes) as asset {}",
        file_name, file_size, asset_id,
    );

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

    let pipeline = AssetPipeline::default().context("failed to create asset pipeline")?;
    let processed = pipeline
        .process_asset(asset)
        .await
        .context("pipeline processing failed")?;

    let shard_store = ShardStore::new();
    let mut shard_hashes: Vec<String> = Vec::with_capacity(processed.shards.len());
    let mut shard_metadata: Vec<ShardMetadata> = Vec::with_capacity(processed.shards.len());

    for shard in &processed.shards {
        let hash_bytes = *blake3::hash(&shard.data).as_bytes();
        let content_hash = ContentHash(hash_bytes);
        shard_store.store(content_hash, shard.data.clone()).await;
        shard_hashes.push(hex::encode(hash_bytes));
        shard_metadata.push(shard.metadata.clone());
    }

    info!(
        "Stored {} shards in local shard store",
        processed.shards.len()
    );

    let map = ShardMap {
        asset_id: asset_id.clone(),
        shard_hashes: shard_hashes.clone(),
        decryption_key: processed.decryption_key,
        shard_count: processed.shards.len(),
        original_size: processed.stats.original_size,
        shard_metadata,
    };

    let maps_dir = shard_maps_dir()?;
    std::fs::create_dir_all(&maps_dir)
        .with_context(|| format!("failed to create {}", maps_dir.display()))?;

    let map_path = maps_dir.join(format!("{}.json", asset_id));
    let map_json =
        serde_json::to_string_pretty(&map).context("failed to serialize shard map")?;
    std::fs::write(&map_path, &map_json)
        .with_context(|| format!("failed to write {}", map_path.display()))?;

    let shards_dir = maps_dir.join(&asset_id);
    std::fs::create_dir_all(&shards_dir)
        .with_context(|| format!("failed to create {}", shards_dir.display()))?;

    for (hash_hex, shard) in shard_hashes.iter().zip(processed.shards.iter()) {
        let shard_path = shards_dir.join(hash_hex);
        std::fs::write(&shard_path, &shard.data)
            .with_context(|| format!("failed to write shard {}", shard_path.display()))?;
    }

    if let Some(ctx) = dist_ctx {
        let shard_pairs: Vec<(ContentHash, Vec<u8>)> = processed
            .shards
            .iter()
            .map(|s| {
                let hash_bytes = *blake3::hash(&s.data).as_bytes();
                (ContentHash(hash_bytes), s.data.clone())
            })
            .collect();
        distribute_shards_to_network(ctx, &shard_pairs, &processed.distributed.placements)
            .await;
    } else {
        debug!("Standalone store: no network available for shard distribution");
    }

    println!("{map_json}");
    info!("Asset ID: {}", asset_id);

    Ok(())
}

/// Run the Fetch subcommand: load a shard map from disk, reconstruct the
/// original file through the reverse pipeline, and write the output.
pub async fn run_fetch(asset_id: String, output: Option<std::path::PathBuf>) -> Result<()> {
    let maps_dir = shard_maps_dir()?;
    let map_path = maps_dir.join(format!("{}.json", asset_id));

    if !map_path.exists() {
        anyhow::bail!(
            "shard map not found at {}. Was the asset stored on this node?",
            map_path.display()
        );
    }

    let map_json = std::fs::read_to_string(&map_path)
        .with_context(|| format!("failed to read {}", map_path.display()))?;
    let map: ShardMap =
        serde_json::from_str(&map_json).context("failed to deserialize shard map")?;

    info!(
        "Loaded shard map for asset {} ({} shards, {} bytes original)",
        map.asset_id, map.shard_count, map.original_size,
    );

    let shards_dir = maps_dir.join(&asset_id);
    let mut shards: Vec<Shard> = Vec::with_capacity(map.shard_count);
    let mut network_fetched = 0usize;

    for (i, hash_hex) in map.shard_hashes.iter().enumerate() {
        let expected_hash = hex::decode(hash_hex)
            .with_context(|| format!("invalid shard hash hex at index {i}"))?;

        let shard_path = shards_dir.join(hash_hex);
        let shard_data = match std::fs::read(&shard_path) {
            Ok(data) => {
                let computed = blake3::hash(&data);
                if computed.as_bytes() != expected_hash.as_slice() {
                    warn!(
                        "Shard {i} BLAKE3 mismatch on disk -- expected {}, got {}",
                        hash_hex,
                        hex::encode(computed.as_bytes()),
                    );
                    None
                } else {
                    Some(data)
                }
            }
            Err(_) => None,
        };

        let shard_data = match shard_data {
            Some(data) => data,
            None => {
                warn!("Shard {i} not available locally, attempting network fetch");
                fetch_shard_from_network(hash_hex, &expected_hash)
                    .await
                    .with_context(|| {
                        format!("shard {i} ({hash_hex}) not available locally or on network")
                    })?
            }
        };

        if shard_data.is_empty() {
            anyhow::bail!("shard {i} ({hash_hex}) returned empty data");
        }

        if !shard_path.exists() {
            network_fetched += 1;
        }

        let metadata = if i < map.shard_metadata.len() {
            map.shard_metadata[i].clone()
        } else {
            ShardMetadata {
                index: i,
                is_parity: false,
                size: shard_data.len(),
                original_size: shard_data.len(),
                hash: hash_hex.clone(),
            }
        };

        shards.push(Shard {
            data: shard_data,
            metadata,
        });
    }

    if network_fetched > 0 {
        info!("Fetched {} shard(s) from network peers", network_fetched);
    }
    info!("Loaded {} shards total", shards.len());

    let processed = ProcessedAsset {
        asset_id: map.asset_id.clone(),
        shards,
        decryption_key: map.decryption_key,
        distributed: DistributedAsset {
            asset_id: map.asset_id.clone(),
            placements: vec![],
            metadata: DistributionMetadata {
                total_shards: map.shard_count,
                networks_used: 0,
                avg_shard_distance: 0.0,
                quality_score: 0.0,
                distributed_at: 0,
            },
        },
        content_hash: [0u8; 32],
        proof_hash: [0u8; 32],
        stats: PipelineStats::default(),
    };

    let pipeline = AssetPipeline::default().context("failed to create asset pipeline")?;
    let reconstructed = pipeline
        .reconstruct_asset(&processed)
        .await
        .context("asset reconstruction failed")?;

    match output {
        Some(ref out_path) => {
            std::fs::write(out_path, &reconstructed)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
            info!(
                "Reconstructed {} bytes -> {}",
                reconstructed.len(),
                out_path.display()
            );
        }
        None => {
            info!("Reconstructed {} bytes", reconstructed.len());
        }
    }

    Ok(())
}

/// Attempt to fetch a shard from connected network peers via IPC daemon.
async fn fetch_shard_from_network(hash_hex: &str, expected_hash: &[u8]) -> Result<Vec<u8>> {
    let client = ipc::IpcClient::new();
    if !client.is_daemon_running().await {
        anyhow::bail!("no daemon running for network shard fetch");
    }

    let resp = client
        .call_ok(
            "shard.fetch",
            serde_json::json!({ "shard_id": hash_hex }),
        )
        .await
        .map_err(|e| anyhow!("IPC shard fetch failed: {e}"))?;

    let shard_hex = resp["data"]
        .as_str()
        .ok_or_else(|| anyhow!("shard.fetch response missing 'data' field"))?;
    let shard_data =
        hex::decode(shard_hex).map_err(|e| anyhow!("invalid shard data hex: {e}"))?;

    let computed = blake3::hash(&shard_data);
    if computed.as_bytes() != expected_hash {
        anyhow::bail!(
            "network shard BLAKE3 mismatch: expected {}, got {}",
            hash_hex,
            hex::encode(computed.as_bytes()),
        );
    }

    info!("Fetched shard {} from network via IPC", hash_hex);
    Ok(shard_data)
}

/// Distribute shards to connected network peers.
async fn distribute_shards_to_network(
    ctx: &ShardDistributionCtx,
    shard_pairs: &[(ContentHash, Vec<u8>)],
    placements: &[blockmatrix::assets::pipeline::distribution::ShardPlacement],
) {
    use blockmatrix::network::shard_distribution::distribute_to_peers;

    let connected_nodes = ctx.network.get_connected_nodes().await;
    let result = distribute_to_peers(
        shard_pairs,
        placements,
        &connected_nodes,
        ctx.shard_transport.as_ref(),
    )
    .await;

    info!(
        "Shard distribution: {} sent, {} kept locally, {} failed",
        result.sent, result.kept_local, result.failed,
    );
}
