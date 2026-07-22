// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Spatial verification for Block-MATRIX: shard commitment + PoSPing protocol.
//!
//! The verification module provides:
//! - **Shard commitment**: BLAKE3 hash anchoring each block to its shard distribution
//! - **PoSPing protocol**: Epoch-seeded bilateral probes that ray-cast into the 3D hash volume
//! - **Consistency checking**: Binary (consistent/inconsistent) verification of spatial evidence
//!
//! Privacy-aware: respects the 2-axis PrivacyMode (Scope gates access, Traceability filters disclosure).
//!
//! # NGauge Integration
//!
//! PoSPing results are reported to NGauge via [`ngauge::streaming::VerificationSnapshot`].
//! The conversion is intentionally not a direct type dependency -- NGauge defines its own
//! wire format for streaming metrics. A node aggregates PoSPing results per epoch and
//! constructs a `VerificationSnapshot` with:
//! - `probes_sent`: number of probes generated this epoch
//! - `probes_passed`: number returning `consistent == true`
//! - `avg_response_time_us`: mean response time across probes
//! - `consistency_ratio`: probes_passed / probes_sent
//! - `epoch`: the epoch number
//!
//! Use [`aggregate_epoch_results`] to compute these values from a batch of [`PoSPingResult`]s.

pub mod min_spec;
pub mod probe;
pub mod response;
pub mod shard_commitment;
pub mod types;

pub use probe::{scope_allows_probe, ProbeGenerator, VolumeBounds};
pub use response::{compute_challenge_response, ConsistencyChecker};
pub use shard_commitment::{
    create_from_distribution, verify_commitment, ShardDistributionMap, ShardPlacement,
};
pub use types::*;

/// Top-level orchestrator for PoSPing verification.
///
/// Manages epoch lifecycle, generates probes, and checks responses.
pub struct PoSPingService {
    probe_generator: ProbeGenerator,
    consistency_checker: ConsistencyChecker,
    config: VerificationConfig,
}

impl PoSPingService {
    /// Create a new PoSPing service with the given configuration.
    pub fn new(config: VerificationConfig) -> Self {
        Self {
            probe_generator: ProbeGenerator::new(config.clone()),
            consistency_checker: ConsistencyChecker::new(config.clone()),
            config,
        }
    }

    /// Start a new verification epoch.
    pub fn start_epoch(&mut self, seed: [u8; 32], epoch_number: u64) {
        self.probe_generator.start_epoch(seed, epoch_number);
    }

    /// Generate probe requests for the current epoch.
    pub fn generate_probes(
        &self,
        prober_position: &hypermesh_lib::MatrixPosition,
        volume_bounds: &VolumeBounds,
    ) -> Vec<(hypermesh_lib::MatrixPosition, PoSPingRequest)> {
        self.probe_generator
            .generate_probes(prober_position, volume_bounds)
    }

    /// Verify a PoSPing response.
    pub fn verify_response(
        &self,
        request: &PoSPingRequest,
        response: &PoSPingResponse,
    ) -> PoSPingResult {
        self.consistency_checker.check(request, response)
    }

    /// Get the current configuration.
    pub fn config(&self) -> &VerificationConfig {
        &self.config
    }
}

/// Aggregated epoch metrics suitable for NGauge reporting.
///
/// Produced by [`aggregate_epoch_results`]. The caller constructs an
/// `ngauge::streaming::VerificationSnapshot` from these fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpochAggregation {
    /// Number of PoSPing probes sent (results.len()).
    pub probes_sent: u32,
    /// Number of probes that returned `consistent == true`.
    pub probes_passed: u32,
    /// Mean response time across all probes, in microseconds.
    pub avg_response_time_us: u64,
    /// Ratio of probes_passed to probes_sent (0.0 to 1.0).
    pub consistency_ratio: f64,
    /// Epoch number (taken from the first result).
    pub epoch: u64,
}

/// Aggregate PoSPing results for an epoch into metrics suitable for NGauge reporting.
///
/// The returned [`EpochAggregation`] maps 1:1 to the fields of
/// `ngauge::streaming::VerificationSnapshot`. This function lives in blockmatrix
/// (not ngauge) so the dependency arrow stays unidirectional: blockmatrix never
/// imports ngauge, and ngauge never imports blockmatrix internals.
///
/// Returns an [`EpochAggregation`] with all-zero fields when `results` is empty.
pub fn aggregate_epoch_results(results: &[PoSPingResult]) -> EpochAggregation {
    if results.is_empty() {
        return EpochAggregation {
            probes_sent: 0,
            probes_passed: 0,
            avg_response_time_us: 0,
            consistency_ratio: 0.0,
            epoch: 0,
        };
    }
    let sent = results.len() as u32;
    let passed = results.iter().filter(|r| r.consistent).count() as u32;
    let total_time: u64 = results.iter().map(|r| r.response_time_us).sum();
    let avg_time = total_time / u64::from(sent);
    let ratio = f64::from(passed) / f64::from(sent);
    let epoch = results.first().map_or(0, |r| r.epoch);
    EpochAggregation {
        probes_sent: sent,
        probes_passed: passed,
        avg_response_time_us: avg_time,
        consistency_ratio: ratio,
        epoch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::MatrixPosition;

    fn pos(x: f64, y: f64, z: f64) -> MatrixPosition {
        MatrixPosition { x, y, z }
    }

    #[test]
    fn service_lifecycle() {
        let mut service = PoSPingService::new(VerificationConfig::default());
        service.start_epoch([0x42; 32], 1);

        let bounds = VolumeBounds::default();
        let probes = service.generate_probes(&pos(0.0, 0.0, 0.0), &bounds);
        assert_eq!(probes.len(), 3); // default probes_per_epoch
    }

    #[test]
    fn service_verify_valid_response() {
        let service = PoSPingService::new(VerificationConfig::default());

        let request = PoSPingRequest {
            prober_position: pos(0.0, 0.0, 0.0),
            target_position: pos(10.0, 20.0, 30.0),
            epoch: 1,
            challenge_nonce: [0x42; 16],
        };

        let chain_head = [0x01; 32];
        let challenge_response = compute_challenge_response(&request.challenge_nonce, &chain_head);

        let response = PoSPingResponse {
            chain_head_hash: chain_head,
            chain_height: 100,
            shard_commitment: None,
            shard_map: None,
            response_time_us: 500,
            challenge_response,
        };

        let result = service.verify_response(&request, &response);
        assert!(result.consistent);
    }

    // -----------------------------------------------------------------------
    // aggregate_epoch_results tests
    // -----------------------------------------------------------------------

    fn make_result(consistent: bool, response_time_us: u64, epoch: u64) -> PoSPingResult {
        PoSPingResult {
            target_position: pos(1.0, 2.0, 3.0),
            consistent,
            shards_checked: 5,
            shards_passed: if consistent { 5 } else { 0 },
            response_time_us,
            epoch,
        }
    }

    #[test]
    fn aggregate_empty_returns_zeroes() {
        let agg = aggregate_epoch_results(&[]);
        assert_eq!(agg.probes_sent, 0);
        assert_eq!(agg.probes_passed, 0);
        assert_eq!(agg.avg_response_time_us, 0);
        assert!((agg.consistency_ratio).abs() < f64::EPSILON);
        assert_eq!(agg.epoch, 0);
    }

    #[test]
    fn aggregate_all_consistent() {
        let results = vec![
            make_result(true, 100, 7),
            make_result(true, 200, 7),
            make_result(true, 300, 7),
        ];
        let agg = aggregate_epoch_results(&results);
        assert_eq!(agg.probes_sent, 3);
        assert_eq!(agg.probes_passed, 3);
        assert_eq!(agg.avg_response_time_us, 200); // (100+200+300)/3
        assert!((agg.consistency_ratio - 1.0).abs() < f64::EPSILON);
        assert_eq!(agg.epoch, 7);
    }

    #[test]
    fn aggregate_mixed_results() {
        let results = vec![
            make_result(true, 500, 42),
            make_result(false, 1000, 42),
            make_result(true, 500, 42),
            make_result(false, 2000, 42),
        ];
        let agg = aggregate_epoch_results(&results);
        assert_eq!(agg.probes_sent, 4);
        assert_eq!(agg.probes_passed, 2);
        assert_eq!(agg.avg_response_time_us, 1000); // (500+1000+500+2000)/4
        assert!((agg.consistency_ratio - 0.5).abs() < f64::EPSILON);
        assert_eq!(agg.epoch, 42);
    }

    #[test]
    fn aggregate_all_inconsistent() {
        let results = vec![make_result(false, 5000, 1), make_result(false, 3000, 1)];
        let agg = aggregate_epoch_results(&results);
        assert_eq!(agg.probes_sent, 2);
        assert_eq!(agg.probes_passed, 0);
        assert_eq!(agg.avg_response_time_us, 4000); // (5000+3000)/2
        assert!((agg.consistency_ratio).abs() < f64::EPSILON);
        assert_eq!(agg.epoch, 1);
    }

    #[test]
    fn aggregate_single_result() {
        let results = vec![make_result(true, 750, 99)];
        let agg = aggregate_epoch_results(&results);
        assert_eq!(agg.probes_sent, 1);
        assert_eq!(agg.probes_passed, 1);
        assert_eq!(agg.avg_response_time_us, 750);
        assert!((agg.consistency_ratio - 1.0).abs() < f64::EPSILON);
        assert_eq!(agg.epoch, 99);
    }
}
