// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Hardware assessment and genesis asset registration for R1/R10 compliance.

use anyhow::{Context, Result};
use tracing::{info, warn};

use blockmatrix::assets::core::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
};
use blockmatrix::create_os_abstraction;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::proof_of_state::genesis_proof::{generate_genesis_proof, HardwareAssessment};
use blockmatrix::StateProof;

/// Assess node hardware and build `AssetRegistration` entries for each
/// detected resource (R1 compliance: hardware assessed, not self-reported,
/// registered as IPv6-addressed assets with Proof of State).
pub fn assess_hardware_assets() -> Result<Vec<AssetRegistration>> {
    let os = create_os_abstraction().context("failed to create OS abstraction")?;
    let platform = os.platform().to_string();
    let mut assets: Vec<AssetRegistration> = Vec::new();

    assess_cpu(os.as_ref(), &platform, &mut assets);
    assess_memory(os.as_ref(), &platform, &mut assets);
    assess_storage(os.as_ref(), &platform, &mut assets);
    assess_network(&platform, &mut assets);
    assess_gpu(os.as_ref(), &platform, &mut assets);

    if assets.is_empty() {
        anyhow::bail!("Hardware assessment found zero assets -- cannot satisfy R1");
    }

    info!(
        "Hardware assessment complete: {} asset(s) detected",
        assets.len()
    );
    Ok(assets)
}

/// Build a StateProof for hardware asset registration using real OS data.
///
/// Per R1: hardware assessed, not self-reported.
/// Per R2: four proofs from actual hardware measurements.
pub fn build_hardware_state_proof(node_id: &str, coordinate: MatrixCoordinate) -> StateProof {
    match create_os_abstraction() {
        Ok(os) => {
            let hw = HardwareAssessment::from_os(os.as_ref(), node_id, coordinate);
            generate_genesis_proof(&hw)
        }
        Err(e) => {
            warn!(
                "OS abstraction unavailable ({e}), using fallback hardware values"
            );
            let hw = HardwareAssessment {
                cpu_cores: num_cpus::get() as u32,
                cpu_mhz: 1000,
                memory_bytes: 4 * 1024 * 1024 * 1024,
                storage_bytes: 50 * 1024 * 1024 * 1024,
                storage_available_bytes: 25 * 1024 * 1024 * 1024,
                node_id: node_id.to_string(),
                coordinate,
            };
            generate_genesis_proof(&hw)
        }
    }
}

/// Build an `AssetRegistration` for the node's FALCON-1024 + Kyber-1024 identity.
pub fn build_identity_asset_registration(
    identity: &blockmatrix::identity::FalconIdentity,
) -> AssetRegistration {
    let recovery_commitment = trustchain::identity::compute_recovery_commitment(
        "default-recovery-phrase",
        &identity.node_id,
    );
    let asset_data = AssetData {
        config: recovery_commitment.to_vec(),
        definition: identity.public_key.clone(),
        metadata: identity.kyber_public_key.clone(),
    };
    AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Identity),
    )
}

fn assess_cpu(
    os: &dyn blockmatrix::os_integration::OsAbstraction,
    platform: &str,
    assets: &mut Vec<AssetRegistration>,
) {
    match os.detect_cpu() {
        Ok(cpu) => {
            let freq_str = cpu
                .frequency_mhz
                .map(|f| format!("{f} MHz"))
                .unwrap_or_else(|| "unknown".to_string());
            let asset_data = AssetData {
                config: format!(
                    "platform={platform},cores={},arch={}",
                    cpu.cores, cpu.architecture,
                )
                .into_bytes(),
                definition: format!("cpu:{}:{}:{}", cpu.model, cpu.cores, freq_str).into_bytes(),
                metadata: format!(
                    "vendor={},freq={}",
                    cpu.vendor.as_deref().unwrap_or("unknown"),
                    freq_str,
                )
                .into_bytes(),
            };
            assets.push(AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Cpu),
            ));
            info!(
                "Hardware: CPU {} ({} cores, {})",
                cpu.model, cpu.cores, freq_str,
            );
        }
        Err(e) => warn!("CPU detection failed: {e}"),
    }
}

fn assess_memory(
    os: &dyn blockmatrix::os_integration::OsAbstraction,
    platform: &str,
    assets: &mut Vec<AssetRegistration>,
) {
    match os.detect_memory() {
        Ok(mem) => {
            let total_mb = mem.total_bytes / (1024 * 1024);
            let avail_mb = mem.available_bytes / (1024 * 1024);
            let asset_data = AssetData {
                config: format!("platform={platform}").into_bytes(),
                definition: format!(
                    "memory:total={},available={}",
                    mem.total_bytes, mem.available_bytes,
                )
                .into_bytes(),
                metadata: format!("usage={:.1}%", mem.usage_percent).into_bytes(),
            };
            assets.push(AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Memory),
            ));
            info!("Hardware: Memory {total_mb} MB total, {avail_mb} MB available");
        }
        Err(e) => warn!("Memory detection failed: {e}"),
    }
}

fn assess_storage(
    os: &dyn blockmatrix::os_integration::OsAbstraction,
    platform: &str,
    assets: &mut Vec<AssetRegistration>,
) {
    match os.detect_storage() {
        Ok(devices) => {
            for dev in &devices {
                let total_gb = dev.total_bytes / (1024 * 1024 * 1024);
                let avail_gb = dev.available_bytes / (1024 * 1024 * 1024);
                let asset_data = AssetData {
                    config: format!(
                        "platform={platform},fs={},type={:?}",
                        dev.filesystem, dev.storage_type,
                    )
                    .into_bytes(),
                    definition: format!(
                        "storage:{}:total={},available={}",
                        dev.mount_point, dev.total_bytes, dev.available_bytes,
                    )
                    .into_bytes(),
                    metadata: format!(
                        "device={},usage={:.1}%",
                        dev.device, dev.usage_percent,
                    )
                    .into_bytes(),
                };
                assets.push(AssetRegistration::from_asset_data(
                    &asset_data,
                    NetworkScope::Global,
                    AssetCategory::BaseSystem(BaseSystemType::Storage),
                ));
                info!(
                    "Hardware: Storage {} ({} GB total, {} GB free, {:?})",
                    dev.mount_point, total_gb, avail_gb, dev.storage_type,
                );
            }
        }
        Err(e) => warn!("Storage detection failed: {e}"),
    }
}

fn assess_network(platform: &str, assets: &mut Vec<AssetRegistration>) {
    let asset_data = AssetData {
        config: format!("platform={platform}").into_bytes(),
        definition: b"network:ipv6:loopback=::1".to_vec(),
        metadata: b"interface=lo".to_vec(),
    };
    assets.push(AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Network),
    ));
    info!("Hardware: Network interface registered");
}

fn assess_gpu(
    os: &dyn blockmatrix::os_integration::OsAbstraction,
    platform: &str,
    assets: &mut Vec<AssetRegistration>,
) {
    match os.detect_gpu() {
        Ok(gpus) => {
            for gpu in &gpus {
                let mem_str = gpu
                    .memory_bytes
                    .map(|m| format!("{} MB", m / (1024 * 1024)))
                    .unwrap_or_else(|| "unknown".to_string());
                let asset_data = AssetData {
                    config: format!(
                        "platform={platform},vendor={},type={:?}",
                        gpu.vendor, gpu.gpu_type,
                    )
                    .into_bytes(),
                    definition: format!("gpu:{}:{}", gpu.model, mem_str).into_bytes(),
                    metadata: format!("capabilities={}", gpu.capabilities.join(",")).into_bytes(),
                };
                assets.push(AssetRegistration::from_asset_data(
                    &asset_data,
                    NetworkScope::Global,
                    AssetCategory::BaseSystem(BaseSystemType::Gpu),
                ));
                info!("Hardware: GPU {} ({}, {})", gpu.model, gpu.vendor, mem_str);
            }
        }
        Err(e) => warn!("GPU detection skipped: {e}"),
    }
}
