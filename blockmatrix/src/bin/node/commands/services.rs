// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Service setup helpers -- dashboard registration, reflector peers, block propagation.

use tracing::{debug, info, warn};

use blockmatrix::assets::core::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
};
use blockmatrix::blockchain::propagation::BlockPropagator;
use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::reflector_pool::ReflectorPool;
use blockmatrix::network::NetworkManager;

use crate::hardware::build_hardware_state_proof;

/// Propagate a newly-created block to connected peers via the BlockPropagator.
pub(super) async fn propagate_block(
    block: &blockmatrix::blockchain::block::Block,
    propagator: &tokio::sync::Mutex<BlockPropagator>,
    network: &NetworkManager,
) {
    let coords = network.get_connected_coordinates().await;
    if coords.is_empty() {
        debug!("No connected peers, skipping block propagation");
        return;
    }

    let result = propagator
        .lock()
        .await
        .propagate_block(block, &coords)
        .await;
    info!(
        "Block #{} propagated to {} peer(s) ({} failed)",
        block.index,
        result.reached_nodes.len(),
        result.failed_nodes.len(),
    );
}

/// Count DNS assets in a block (used for logging during propagation).
pub(super) fn count_dns_assets_in_block(
    block: &blockmatrix::blockchain::block::Block,
) -> usize {
    block
        .get_assets()
        .iter()
        .filter(|asset| {
            matches!(
                asset.category,
                AssetCategory::BaseSystem(BaseSystemType::Dns)
            )
        })
        .count()
}

pub(super) async fn register_reflector_peers(
    network: &std::sync::Arc<NetworkManager>,
    reflector_pool: &std::sync::Arc<tokio::sync::Mutex<ReflectorPool>>,
    network_id: &str,
    has_bootstrap_peers: bool,
    is_reflector: bool,
    privacy_mode: PrivacyMode,
) {
    if !has_bootstrap_peers && !is_reflector {
        return;
    }

    let discovered_peers = network.get_connected_nodes().await;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut rp = reflector_pool.lock().await;
    for peer in &discovered_peers {
        let reflector = blockmatrix::network::reflector_pool::Reflector {
            node_id: peer.node_id.clone(),
            position: hypermesh_lib::MatrixPosition {
                x: peer.coordinate.x as f64,
                y: peer.coordinate.y as f64,
                z: peer.coordinate.z as f64,
            },
            last_seen: now_secs,
            block_height: 0,
            health_score: 1.0,
            privacy_mode,
        };
        rp.register_reflector(network_id, reflector);
        info!(
            "Registered peer {} as reflector for {}",
            &peer.node_id[..8.min(peer.node_id.len())],
            network_id,
        );
    }
}

/// Register default system dashboard as a blockchain asset if none exists.
pub(super) async fn register_default_dashboard(
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    nid: &str,
    coord: MatrixCoordinate,
) {
    use blockmatrix::dashboard::deploy;

    let chain = bootstrap.blockchain().get_chain().await;
    if deploy::find_active_dashboard(&chain).is_some() {
        return;
    }

    info!("Registering default system dashboard as blockchain asset...");

    let mut files = std::collections::BTreeMap::new();

    let ui_dist_candidates = [
        std::path::PathBuf::from("ui/frontend/dist"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("../ui/frontend/dist")))
            .unwrap_or_default(),
        data_dir.join("ui/dist"),
    ];

    let mut loaded_ui = false;
    for ui_dist in &ui_dist_candidates {
        if ui_dist.join("index.html").exists() {
            match deploy::collect_dir_files(ui_dist) {
                Ok(ui_files) if !ui_files.is_empty() => {
                    info!("Loading UI from {}", ui_dist.display());
                    for (path, content) in &ui_files {
                        files.insert(format!("private/{path}"), content.clone());
                    }
                    loaded_ui = true;
                    break;
                }
                _ => {}
            }
        }
    }

    if !loaded_ui {
        info!("Built UI not found, using embedded fallback dashboard");
        files.insert(
            "private/index.html".to_string(),
            blockmatrix::dashboard::default::DEFAULT_PRIVATE_HTML
                .as_bytes()
                .to_vec(),
        );
    }

    files.insert(
        "public/index.html".to_string(),
        blockmatrix::dashboard::default::DEFAULT_PUBLIC_HTML
            .as_bytes()
            .to_vec(),
    );

    let bundle = deploy::bundle_files(&files);

    let manifest_toml = r#"[dashboard]
name = "default"
version = "1.0.0"
description = "Default HyperMesh node dashboard"
domain = "localhost.hypermesh"

[access]
public = "public"
private = "private"
"#;

    let asset_data = AssetData {
        config: b"DASHBOARD:DEPLOY:default".to_vec(),
        definition: bundle.clone(),
        metadata: manifest_toml.as_bytes().to_vec(),
    };
    let registration = AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Dashboard),
    );
    let content_hash = registration.content_hash;
    let state_proof = build_hardware_state_proof(nid, coord);

    match bootstrap
        .blockchain()
        .register_asset_record(registration, &state_proof)
        .await
    {
        Ok(block) => {
            if let Err(e) = deploy::store_dashboard_bundle(
                data_dir,
                &content_hash,
                manifest_toml,
                &bundle,
            ) {
                warn!("Failed to store dashboard bundle: {}", e);
            }
            info!(
                "Default dashboard registered as asset (block #{}, hash {})",
                block.index,
                hex::encode(content_hash)
            );
        }
        Err(e) => warn!("Failed to register default dashboard: {}", e),
    }
}
