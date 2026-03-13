// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Node bootstrap and resume logic -- genesis block creation and state recovery.

use anyhow::{Context, Result};
use tracing::{info, warn};

use blockmatrix::bootstrap::{LocalhostCertificate, NodeBootstrap};
use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::persistence::{BlockQuery, PersistenceConfig, PersistenceManager};

use crate::hardware::{
    assess_hardware_assets, build_hardware_state_proof, build_identity_asset_registration,
};

pub(crate) async fn resume_node(
    data_dir: &std::path::Path,
    nid: &str,
    coord: MatrixCoordinate,
) -> Result<(NodeBootstrap, PersistenceManager)> {
    info!(
        "Found persisted state at {}, resuming node",
        data_dir.display()
    );

    let persistence_config = PersistenceConfig {
        storage_dir: data_dir.to_path_buf(),
        enable_background: true,
        ..PersistenceConfig::default()
    };
    let persistence = PersistenceManager::new(persistence_config, nid.to_string())
        .await
        .context("failed to initialize persistence manager")?;

    let report = persistence.recover().await.context("recovery failed")?;
    info!(
        "Recovery complete: status={:?}, blocks_recovered={}, wal_replayed={}",
        report.status, report.stats.blocks_recovered, report.stats.wal_entries_replayed,
    );

    let genesis_block = persistence
        .load_block(BlockQuery::ByIndex(0))
        .await
        .context("failed to load genesis block")?
        .ok_or_else(|| anyhow::anyhow!("persisted state exists but genesis block missing"))?;

    let stats = persistence.get_stats().await;
    let chain_height = stats.block_count.saturating_sub(1);

    let blocks = if chain_height > 0 {
        let mut all_blocks = vec![genesis_block.clone()];
        for idx in 1..=chain_height {
            if let Some(block) = persistence
                .load_block(BlockQuery::ByIndex(idx))
                .await
                .context("failed to load block")?
            {
                all_blocks.push(block);
            }
        }
        all_blocks
    } else {
        vec![genesis_block.clone()]
    };

    info!("Loaded {} blocks from disk", blocks.len());

    let blockchain = std::sync::Arc::new(
        NodeBlockchain::from_blocks(coord, blocks)
            .map_err(|e| anyhow::anyhow!("failed to reconstruct blockchain: {}", e))?,
    );

    let cert_path = data_dir.join(nid).join("certificate.json");
    let localhost_cert = if cert_path.exists() {
        let cert_json = std::fs::read_to_string(&cert_path)
            .with_context(|| format!("failed to read {}", cert_path.display()))?;
        serde_json::from_str::<LocalhostCertificate>(&cert_json)
            .context("failed to deserialize certificate")?
    } else {
        warn!("Certificate not found on disk, generating fresh one");
        NodeBootstrap::generate_fresh_certificate()?
    };

    let bootstrap =
        NodeBootstrap::resume(coord, blockchain, genesis_block, localhost_cert).await?;
    Ok((bootstrap, persistence))
}

pub(crate) async fn fresh_boot(
    data_dir: &std::path::Path,
    nid: &str,
    coord: MatrixCoordinate,
) -> Result<(NodeBootstrap, PersistenceManager)> {
    info!(
        "No persisted state found, initializing fresh node at ({}, {}, {})",
        coord.x, coord.y, coord.z
    );

    let bootstrap = NodeBootstrap::initialize(coord).await?;

    let persistence_config = PersistenceConfig {
        storage_dir: data_dir.to_path_buf(),
        enable_background: true,
        ..PersistenceConfig::default()
    };
    let persistence = PersistenceManager::new(persistence_config, nid.to_string())
        .await
        .context("failed to initialize persistence manager")?;

    persistence
        .save_block(bootstrap.genesis_block())
        .await
        .context("failed to persist genesis block")?;

    let cert_path = data_dir.join(nid).join("certificate.json");
    let cert_json = serde_json::to_string_pretty(bootstrap.localhost_certificate())
        .context("failed to serialize certificate")?;
    std::fs::write(&cert_path, &cert_json)
        .with_context(|| format!("failed to write {}", cert_path.display()))?;

    info!(
        "Persisted genesis block and certificate to {}",
        data_dir.display()
    );

    // === R1/R10: Load identity and assess hardware for genesis asset registration ===
    let identity_dir = data_dir.join(nid).join("identity");
    let falcon_identity =
        blockmatrix::identity::FalconIdentity::load_or_create(&identity_dir)?;
    info!(
        "Genesis identity: {}... (FALCON-1024 + Kyber-1024)",
        &falcon_identity.node_id[..16]
    );

    info!("Assessing node hardware for asset registration (R1)...");
    match assess_hardware_assets() {
        Ok(mut hw_assets) => {
            hw_assets.push(build_identity_asset_registration(&falcon_identity));

            let state_proof = build_hardware_state_proof(nid, coord);
            match bootstrap
                .blockchain()
                .register_asset_records(hw_assets, &state_proof)
                .await
            {
                Ok(block) => {
                    info!(
                        "Registered hardware + identity assets in block #{} (hash: {})",
                        block.index,
                        &block.hash[..16],
                    );
                    info!(
                        "Identity registered as blockchain asset (node_id: {})",
                        &falcon_identity.node_id[..16],
                    );
                    if let Err(e) = persistence.save_block(&block).await {
                        warn!("Failed to persist hardware asset block: {e}");
                    }
                }
                Err(e) => warn!("Failed to register hardware assets: {e}"),
            }
        }
        Err(e) => warn!("Hardware assessment failed: {e}"),
    }

    Ok((bootstrap, persistence))
}
