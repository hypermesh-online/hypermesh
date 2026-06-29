// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Universal system-asset adapter.
//!
//! Hoists REAL host hardware (CPU / memory / storage) into HyperMesh assets and
//! binds them to a working Proof-of-State built from live, probed metrics. This
//! is a generic system-asset module: it implements the existing
//! [`crate::assets::core::AssetAdapter`] trait against the host's actual
//! hardware, cross-platform (UNIX + Windows), with no fork of any external OS.
//!
//! Layout:
//!   - [`probe`]         -- the [`HardwareProbe`] trait + metric structs.
//!   - [`os`]            -- per-OS probes ([`os::unix`] / [`os::windows`]).
//!   - [`system_adapter`] -- [`SystemAssetAdapter`] implementing `AssetAdapter`.
//!   - [`proof_binding`]  -- [`ProofBoundAsset`]: a `StateProof` welded to an
//!                           allocation, re-validated against live hardware.

pub mod os;
pub mod probe;
pub mod proof_binding;
pub mod system_adapter;

pub use probe::{CpuMetrics, HardwareProbe, MemoryMetrics, StorageMetrics};
pub use proof_binding::{build_state_proof_from_hardware, ProofBoundAsset};
pub use system_adapter::SystemAssetAdapter;

/// The host hardware probe for the active target (UNIX or Windows).
#[cfg(unix)]
pub use os::unix::UnixHardwareProbe as HostProbe;
#[cfg(windows)]
pub use os::windows::WindowsHardwareProbe as HostProbe;

#[cfg(unix)]
pub use os::unix::UnixHardwareProbe;
#[cfg(windows)]
pub use os::windows::WindowsHardwareProbe;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{
        AssetAdapter, AssetCategory, AssetData, AssetRegistration, AssetType, BaseSystemType,
        NetworkScope,
    };
    use std::time::Duration;

    /// Construct the host probe for whichever OS is running the tests.
    fn host_probe() -> HostProbe {
        HostProbe::new()
    }

    /// A storage-category asset id (used for ProofBoundAsset binding tests).
    fn storage_asset_id() -> AssetRegistration {
        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        AssetRegistration::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Storage),
        )
    }

    #[tokio::test]
    async fn probe_this_machine_reports_real_hardware() {
        let probe = host_probe();

        let cpu = probe.probe_cpu().await.expect("test: cpu probe");
        let mem = probe.probe_memory().await.expect("test: memory probe");
        let storage = probe.probe_storage().await.expect("test: storage probe");

        // Real values from the real machine.
        assert!(cpu.logical_cores > 0, "cpu cores must be > 0");
        assert!(mem.total_bytes > 0, "total memory must be > 0");
        assert!(
            storage.total_bytes > 1024 * 1024 * 1024,
            "total disk must exceed 1GB, got {} bytes",
            storage.total_bytes
        );

        // Surface the probed numbers in the test log for the report.
        println!(
            "PROBE[{}]: cores={} freq_mhz={} util%={:.1} | mem_total={} mem_used={} swap_total={} | disk_total={} disk_avail={}",
            std::env::consts::OS,
            cpu.logical_cores,
            cpu.frequency_mhz,
            cpu.utilization_percent,
            mem.total_bytes,
            mem.used_bytes,
            mem.total_swap_bytes,
            storage.total_bytes,
            storage.available_bytes,
        );
    }

    #[tokio::test]
    async fn state_proof_from_real_metrics_validates() {
        let proof = build_state_proof_from_hardware("hypermesh-systemtest-node")
            .await
            .expect("test: build state proof from hardware");

        assert!(
            proof.validate(),
            "PoS proof built from real metrics must validate"
        );
        assert!(
            proof.space_proof.total_storage > 0,
            "space proof must commit real storage"
        );
        assert!(
            proof.work_proof.computational_power > 0,
            "work proof must reflect real compute"
        );

        println!(
            "PROOF: stake={} compute={} storage_total={} storage_used={}",
            proof.stake_proof.stake_amount,
            proof.work_proof.computational_power,
            proof.space_proof.total_storage,
            proof.space_proof.total_size,
        );
    }

    #[tokio::test]
    async fn proof_bound_asset_validates_when_fresh() {
        let adapter = SystemAssetAdapter::storage(host_probe());
        let asset_id = storage_asset_id();

        let bound = ProofBoundAsset::generate(
            asset_id,
            "hypermesh-systemtest-node",
            Duration::from_secs(300),
        )
        .await
        .expect("test: generate proof-bound asset");

        assert!(!bound.is_expired(), "fresh binding must not be expired");
        let valid = bound
            .validate_current(&adapter)
            .await
            .expect("test: validate_current");
        assert!(valid, "fresh proof-bound asset must validate against live hardware");
    }

    #[tokio::test]
    async fn proof_bound_asset_fails_when_expired() {
        let adapter = SystemAssetAdapter::storage(host_probe());
        let asset_id = storage_asset_id();

        let proof = build_state_proof_from_hardware("hypermesh-systemtest-node")
            .await
            .expect("test: build proof");

        // Bind with a zero lifetime so it is already expired.
        let bound = ProofBoundAsset::from_proof(asset_id, proof, Duration::from_secs(0));

        assert!(bound.is_expired(), "zero-lifetime binding must be expired");
        let valid = bound
            .validate_current(&adapter)
            .await
            .expect("test: validate_current on expired");
        assert!(!valid, "expired proof-bound asset must fail validation");
    }

    #[tokio::test]
    async fn proof_bound_asset_fails_on_wrong_asset_kind() {
        // Bind to a STORAGE asset but validate against a CPU adapter -> mismatch.
        let cpu_adapter = SystemAssetAdapter::cpu(host_probe());
        let storage_id = storage_asset_id();

        let bound = ProofBoundAsset::generate(
            storage_id,
            "hypermesh-systemtest-node",
            Duration::from_secs(300),
        )
        .await
        .expect("test: generate");

        let valid = bound
            .validate_current(&cpu_adapter)
            .await
            .expect("test: validate_current cross-kind");
        assert!(
            !valid,
            "proof bound to Storage must not validate on a Cpu adapter"
        );
    }

    #[tokio::test]
    async fn adapter_reports_live_resource_usage_per_kind() {
        let asset_id = storage_asset_id();

        let cpu = SystemAssetAdapter::cpu(host_probe());
        let usage = cpu
            .get_resource_usage(&asset_id)
            .await
            .expect("test: cpu usage");
        assert!(usage.cpu_usage.is_some(), "cpu adapter fills cpu_usage");
        assert!(usage.memory_usage.is_none());

        let mem = SystemAssetAdapter::memory(host_probe());
        let usage = mem
            .get_resource_usage(&asset_id)
            .await
            .expect("test: mem usage");
        assert!(
            usage.memory_usage.as_ref().is_some_and(|m| m.total_bytes > 0),
            "memory adapter fills real memory_usage"
        );

        let storage = SystemAssetAdapter::storage(host_probe());
        let usage = storage
            .get_resource_usage(&asset_id)
            .await
            .expect("test: storage usage");
        assert!(
            usage
                .storage_usage
                .as_ref()
                .is_some_and(|s| s.total_bytes > 1024 * 1024 * 1024),
            "storage adapter fills real storage_usage"
        );
    }

    #[tokio::test]
    async fn adapter_validates_real_proof_and_health() {
        let adapter = SystemAssetAdapter::cpu(host_probe());

        let proof = build_state_proof_from_hardware("hypermesh-systemtest-node")
            .await
            .expect("test: proof");

        let ok = adapter
            .validate_state_proof(&proof)
            .await
            .expect("test: validate_state_proof");
        assert!(ok, "real proof must validate against live hardware");

        let health = adapter.health_check().await.expect("test: health");
        assert!(health.healthy, "cpu adapter health must be healthy");

        // Wrong kind / non-base type construction is rejected.
        assert!(
            SystemAssetAdapter::new(AssetType::Network, host_probe()).is_err(),
            "non Cpu/Memory/Storage kind must be rejected"
        );
    }
}
