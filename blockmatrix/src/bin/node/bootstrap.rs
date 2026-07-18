// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Node bootstrap and resume logic -- genesis block creation and state recovery.

use anyhow::{Context, Result};
use tracing::{info, warn};

use blockmatrix::bootstrap::{LocalhostCertificate, NodeBootstrap};
use blockmatrix::blockchain::block::Block;
use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::create_os_abstraction;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::persistence::{BlockQuery, PersistenceConfig, PersistenceManager};
use blockmatrix::proof_of_state::genesis_proof::{
    evaluate_continuity, recorded_fingerprint_hex, ContinuityDecision,
};

use crate::hardware::{
    assess_hardware_assets, build_hardware_state_proof, build_identity_asset_registration,
};

/// Minimum independent hardware sources required under `--require-hardware-auth`.
const MIN_HARDWARE_SOURCES: usize = 2;

/// Verify the live device matches the device recorded in the genesis block.
///
/// Device-auth invariant: a copied identity directory carried to a different
/// physical machine will produce a DIFFERENT live fingerprint than the one
/// recorded at genesis on the original machine. Under `--require-hardware-auth`
/// this hard-fails startup, rejecting the copy.
///
/// The device fingerprint is always captured; enforcement is gated by
/// `require_hardware_auth` so a normal dev run keeps working.
fn verify_device_continuity(genesis_block: &Block, require_hardware_auth: bool) -> Result<()> {
    let os = match create_os_abstraction() {
        Ok(os) => os,
        Err(e) => {
            if require_hardware_auth {
                anyhow::bail!(
                    "--require-hardware-auth: OS abstraction unavailable, cannot verify device continuity: {e}"
                );
            }
            warn!("Device continuity check skipped (no OS abstraction): {e}");
            return Ok(());
        }
    };

    let live = os.device_fingerprint();
    let live_hex = live.hex();

    let recorded = genesis_block
        .entries
        .first()
        .and_then(|e| recorded_fingerprint_hex(&e.state_proof));

    // Pure decision (unit-tested in genesis_proof) — the reject-a-copy
    // behaviour lives there; this only reports + enforces.
    let decision = evaluate_continuity(recorded.as_deref(), &live_hex);
    let short = |s: &str| s[..16.min(s.len())].to_string();

    if decision.permits_startup(require_hardware_auth) {
        match &decision {
            ContinuityDecision::Match => info!(
                "Device continuity OK: live fingerprint {}... matches genesis ({} sources)",
                short(&live_hex),
                live.source_count
            ),
            ContinuityDecision::Mismatch { recorded, .. } => warn!(
                "Device continuity FAILED: live fingerprint {}... != genesis {}... \
                 (copied identity on a different machine?) — allowed \
                 (run with --require-hardware-auth to enforce)",
                short(&live_hex),
                short(recorded)
            ),
            ContinuityDecision::NoRecordedFingerprint => warn!(
                "Genesis predates the device-auth binding (no recorded fingerprint) \
                 — continuity not enforced"
            ),
        }
        return Ok(());
    }

    // Enforcement on + not a Match → reject startup.
    match decision {
        ContinuityDecision::Mismatch { recorded, .. } => anyhow::bail!(
            "Device continuity FAILED: live fingerprint {}... != genesis {}... \
             (copied identity on a different machine?)",
            short(&live_hex),
            short(&recorded)
        ),
        ContinuityDecision::NoRecordedFingerprint => anyhow::bail!(
            "--require-hardware-auth: genesis predates the device-auth binding \
             (no recorded fingerprint); re-genesis required to enforce continuity"
        ),
        ContinuityDecision::Match => unreachable!("Match always permits startup"),
    }
}

/// Enforce the minimum-independent-sources policy under `--require-hardware-auth`.
fn enforce_min_sources(require_hardware_auth: bool) -> Result<()> {
    if !require_hardware_auth {
        return Ok(());
    }
    let os = create_os_abstraction()
        .context("--require-hardware-auth: OS abstraction required for hardware sources")?;
    let fp = os.device_fingerprint();
    if !fp.has_min_sources(MIN_HARDWARE_SOURCES) {
        anyhow::bail!(
            "--require-hardware-auth: only {} independent hardware source(s) available \
             (need >= {}). DMI is often root-only; run as root or drop the flag.",
            fp.source_count,
            MIN_HARDWARE_SOURCES
        );
    }
    info!(
        "Hardware auth: {} independent source(s) satisfy minimum {}",
        fp.source_count, MIN_HARDWARE_SOURCES
    );
    Ok(())
}

pub(crate) async fn resume_node(
    data_dir: &std::path::Path,
    nid: &str,
    coord: MatrixCoordinate,
    require_hardware_auth: bool,
) -> Result<(NodeBootstrap, PersistenceManager)> {
    info!(
        "Found persisted state at {}, resuming node",
        data_dir.display()
    );

    enforce_min_sources(require_hardware_auth)?;

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

    // Device-auth continuity gate: reject a copied identity dir on a machine
    // whose live fingerprint does not match the genesis-recorded fingerprint.
    verify_device_continuity(&genesis_block, require_hardware_auth)?;

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

    // H3: load this node's FALCON identity so the reconstructed chain can sign
    // locally-produced blocks' proof envelopes (single local-write chokepoint).
    let identity_dir = data_dir.join(nid).join("identity");
    let falcon_identity: std::sync::Arc<dyn hypermesh_lib::NodeSigner + Send + Sync> =
        std::sync::Arc::new(
            blockmatrix::identity::FalconIdentity::load_or_create(&identity_dir)
                .context("failed to load FALCON identity for block signing")?,
        );

    // H3: attach the signer so `add_block` FALCON-signs produced proofs.
    // Block-accept validation uses `default()` StateRequirements — PoStake is
    // AUTHORIZATION (to whom an asset belongs), not a numeric magnitude, so the
    // hardening is the FALCON signature + signer↔owner binding, not a raised
    // stake floor.
    let blockchain = std::sync::Arc::new(
        NodeBlockchain::from_blocks(coord, blocks)
            .map_err(|e| anyhow::anyhow!("failed to reconstruct blockchain: {}", e))?
            .with_signer(falcon_identity),
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
    device_node_id: &str,
    require_hardware_auth: bool,
) -> Result<(NodeBootstrap, PersistenceManager)> {
    info!(
        "No persisted state found, initializing fresh node at ({}, {}, {})",
        coord.x, coord.y, coord.z
    );

    // Fail closed on insufficient hardware sources when enforcement is on.
    enforce_min_sources(require_hardware_auth)?;

    // H3: load this node's FALCON identity up front so the fresh chain can
    // FALCON-sign locally-produced block proof envelopes (single local-write
    // signing chokepoint). `FalconIdentity` is not `Clone`, so the downstream
    // hardware-asset registration re-loads it (idempotent `load_or_create`
    // reads the same persisted keys).
    let signing_identity_dir = data_dir.join(nid).join("identity");
    let signer: std::sync::Arc<dyn hypermesh_lib::NodeSigner + Send + Sync> =
        std::sync::Arc::new(blockmatrix::identity::FalconIdentity::load_or_create(
            &signing_identity_dir,
        )?);

    // Genesis bound to the canonical device identity (collapses the three
    // historical node IDs into `device_node_id`). The device fingerprint is
    // captured inside the genesis proofs unconditionally. H3: attach the signer
    // so `add_block` signs produced proofs.
    let bootstrap = NodeBootstrap::initialize_with_identity_and_signer(
        coord,
        device_node_id,
        signer,
    )
    .await?;

    if let Some(fp) = recorded_fp_of(bootstrap.genesis_block()) {
        info!(
            "Genesis device fingerprint recorded: {}... (node_id {}...)",
            &fp[..16.min(fp.len())],
            &device_node_id[..16.min(device_node_id.len())]
        );
    }

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

            // Collapsed node ID: hardware asset proof uses the canonical
            // device node ID, not the coord-derived data-dir alias.
            let state_proof = build_hardware_state_proof(&falcon_identity.node_id, coord);
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

/// Read the device fingerprint hex recorded in a genesis block (for logging).
fn recorded_fp_of(genesis_block: &Block) -> Option<String> {
    genesis_block
        .entries
        .first()
        .and_then(|e| recorded_fingerprint_hex(&e.state_proof))
}
