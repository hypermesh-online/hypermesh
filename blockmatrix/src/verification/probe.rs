// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Epoch-seeded probe generation with privacy-aware scope gating.
//!
//! Probe targets are deterministic: `BLAKE3(epoch_seed || prober_position || epoch_counter)`.
//! Both parties can verify a probe was legitimately assigned.
//! Scope is checked FIRST: Bounded targets only accept federation members.

use hypermesh_lib::{AccessScope, MatrixPosition, PrivacyMode};
use std::time::SystemTime;

use super::types::{PoSPingRequest, ProbeEpoch, VerificationConfig};

/// Generates deterministic probe targets from epoch entropy.
pub struct ProbeGenerator {
    config: VerificationConfig,
    current_epoch: Option<ProbeEpoch>,
}

impl ProbeGenerator {
    pub fn new(config: VerificationConfig) -> Self {
        Self {
            config,
            current_epoch: None,
        }
    }

    /// Start a new epoch with the given seed.
    pub fn start_epoch(&mut self, seed: [u8; 32], epoch_number: u64) {
        self.current_epoch = Some(ProbeEpoch {
            seed,
            epoch_number,
            started_at: SystemTime::now(),
            duration_secs: self.config.epoch_duration_secs,
        });
    }

    /// Get the current epoch, if active.
    pub fn current_epoch(&self) -> Option<&ProbeEpoch> {
        self.current_epoch.as_ref()
    }

    /// Compute the probe target position for a given prober and probe index.
    ///
    /// `probe_target = BLAKE3(epoch_seed || prober_position_bytes || probe_index)`
    /// The result is mapped into a MatrixPosition within the given volume bounds.
    pub fn compute_target(
        &self,
        prober_position: &MatrixPosition,
        probe_index: u8,
        volume_bounds: &VolumeBounds,
    ) -> Option<MatrixPosition> {
        let epoch = self.current_epoch.as_ref()?;

        let mut hasher = blake3::Hasher::new();
        hasher.update(&epoch.seed);
        hasher.update(&prober_position.x.to_le_bytes());
        hasher.update(&prober_position.y.to_le_bytes());
        hasher.update(&prober_position.z.to_le_bytes());
        hasher.update(&[probe_index]);
        let hash = hasher.finalize();
        let bytes = hash.as_bytes();

        // Map first 24 bytes into 3 f64 coordinates within bounds
        let x = map_to_range(&bytes[0..8], volume_bounds.min_x, volume_bounds.max_x);
        let y = map_to_range(&bytes[8..16], volume_bounds.min_y, volume_bounds.max_y);
        let z = map_to_range(&bytes[16..24], volume_bounds.min_z, volume_bounds.max_z);

        Some(MatrixPosition { x, y, z })
    }

    /// Generate probe requests for this epoch.
    ///
    /// Returns up to `probes_per_epoch` requests, each targeting a deterministic
    /// position in the matrix volume.
    pub fn generate_probes(
        &self,
        prober_position: &MatrixPosition,
        volume_bounds: &VolumeBounds,
    ) -> Vec<(MatrixPosition, PoSPingRequest)> {
        let epoch = match &self.current_epoch {
            Some(e) => e,
            None => return vec![],
        };

        (0..self.config.probes_per_epoch)
            .filter_map(|i| {
                let target = self.compute_target(prober_position, i, volume_bounds)?;
                let mut nonce = [0u8; 16];
                // Deterministic nonce from epoch + probe index
                let nonce_hash = blake3::hash(&[&epoch.seed[..], &[i]].concat());
                nonce.copy_from_slice(&nonce_hash.as_bytes()[..16]);

                let request = PoSPingRequest {
                    prober_position: *prober_position,
                    target_position: target,
                    epoch: epoch.epoch_number,
                    challenge_nonce: nonce,
                };
                Some((target, request))
            })
            .collect()
    }
}

/// Bounds of the known matrix volume for mapping probe targets.
#[derive(Debug, Clone)]
pub struct VolumeBounds {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub min_z: f64,
    pub max_z: f64,
}

impl Default for VolumeBounds {
    fn default() -> Self {
        Self {
            min_x: -1000.0,
            max_x: 1000.0,
            min_y: -1000.0,
            max_y: 1000.0,
            min_z: -1000.0,
            max_z: 1000.0,
        }
    }
}

/// Check whether a prober is allowed to probe a target based on scope.
///
/// Rule 1 -- Scope gate (checked FIRST):
/// - `target.scope == Unbounded` -> always allowed
/// - `target.scope == Bounded` -> only if prober is in target's federation
pub fn scope_allows_probe(target_privacy: &PrivacyMode, prober_in_federation: bool) -> bool {
    match target_privacy.scope {
        AccessScope::Unbounded => true,
        AccessScope::Bounded => prober_in_federation,
    }
}

/// Map 8 bytes to a f64 in [min, max] range.
fn map_to_range(bytes: &[u8], min: f64, max: f64) -> f64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    let raw = u64::from_le_bytes(arr);
    let normalized = raw as f64 / u64::MAX as f64; // 0.0 to 1.0
    min + normalized * (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f64, y: f64, z: f64) -> MatrixPosition {
        MatrixPosition { x, y, z }
    }

    #[test]
    fn probe_generation_is_deterministic() {
        let config = VerificationConfig::default();
        let mut gen = ProbeGenerator::new(config);
        gen.start_epoch([0x42; 32], 1);

        let bounds = VolumeBounds::default();
        let prober = pos(10.0, 20.0, 30.0);

        let probes1 = gen.generate_probes(&prober, &bounds);
        let probes2 = gen.generate_probes(&prober, &bounds);

        assert_eq!(probes1.len(), probes2.len());
        for (a, b) in probes1.iter().zip(probes2.iter()) {
            assert_eq!(a.0.x, b.0.x);
            assert_eq!(a.0.y, b.0.y);
            assert_eq!(a.0.z, b.0.z);
            assert_eq!(a.1.challenge_nonce, b.1.challenge_nonce);
        }
    }

    #[test]
    fn different_epochs_produce_different_targets() {
        let config = VerificationConfig::default();
        let mut gen = ProbeGenerator::new(config);
        let bounds = VolumeBounds::default();
        let prober = pos(10.0, 20.0, 30.0);

        gen.start_epoch([0x42; 32], 1);
        let probes1 = gen.generate_probes(&prober, &bounds);

        gen.start_epoch([0x43; 32], 2);
        let probes2 = gen.generate_probes(&prober, &bounds);

        // At least one target should differ
        let any_different = probes1
            .iter()
            .zip(probes2.iter())
            .any(|(a, b)| a.0.x != b.0.x || a.0.y != b.0.y || a.0.z != b.0.z);
        assert!(
            any_different,
            "different epochs must produce different targets"
        );
    }

    #[test]
    fn targets_within_volume_bounds() {
        let config = VerificationConfig::default();
        let mut gen = ProbeGenerator::new(config);
        gen.start_epoch([0xAB; 32], 100);

        let bounds = VolumeBounds {
            min_x: -50.0,
            max_x: 50.0,
            min_y: -50.0,
            max_y: 50.0,
            min_z: -50.0,
            max_z: 50.0,
        };
        let prober = pos(0.0, 0.0, 0.0);

        let probes = gen.generate_probes(&prober, &bounds);
        for (target, _) in &probes {
            assert!(target.x >= bounds.min_x && target.x <= bounds.max_x);
            assert!(target.y >= bounds.min_y && target.y <= bounds.max_y);
            assert!(target.z >= bounds.min_z && target.z <= bounds.max_z);
        }
    }

    #[test]
    fn no_probes_without_epoch() {
        let config = VerificationConfig::default();
        let gen = ProbeGenerator::new(config);
        let bounds = VolumeBounds::default();
        let prober = pos(0.0, 0.0, 0.0);
        assert!(gen.generate_probes(&prober, &bounds).is_empty());
    }

    #[test]
    fn scope_gate_unbounded_always_allows() {
        assert!(scope_allows_probe(&PrivacyMode::ANONYMOUS, false));
        assert!(scope_allows_probe(&PrivacyMode::PUBLIC, false));
        assert!(scope_allows_probe(&PrivacyMode::ANONYMOUS, true));
        assert!(scope_allows_probe(&PrivacyMode::PUBLIC, true));
    }

    #[test]
    fn scope_gate_bounded_requires_federation() {
        assert!(!scope_allows_probe(&PrivacyMode::PRIVATE, false));
        assert!(scope_allows_probe(&PrivacyMode::PRIVATE, true));
    }

    #[test]
    fn probes_per_epoch_respected() {
        let mut config = VerificationConfig::default();
        config.probes_per_epoch = 5;
        let mut gen = ProbeGenerator::new(config);
        gen.start_epoch([0x01; 32], 1);
        let bounds = VolumeBounds::default();
        let prober = pos(0.0, 0.0, 0.0);
        assert_eq!(gen.generate_probes(&prober, &bounds).len(), 5);
    }
}
