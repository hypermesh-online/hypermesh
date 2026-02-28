// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Types for spatial verification (shard commitment + PoSPing protocol).

use hypermesh_lib::{MatrixPosition, NodeId};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Epoch state for deterministic probe selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeEpoch {
    /// Epoch entropy seed: BLAKE3(network_entropy || epoch_number).
    pub seed: [u8; 32],
    /// Monotonically increasing epoch counter.
    pub epoch_number: u64,
    /// Wall-clock start of this epoch.
    #[serde(with = "system_time_serde")]
    pub started_at: SystemTime,
    /// Epoch length in seconds (default 60).
    pub duration_secs: u64,
}

/// A probe request sent from prober to target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoSPingRequest {
    pub prober_position: MatrixPosition,
    pub target_position: MatrixPosition,
    pub epoch: u64,
    pub challenge_nonce: [u8; 16],
}

/// Target's response to a PoSPing probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoSPingResponse {
    pub chain_head_hash: [u8; 32],
    pub chain_height: u64,
    pub shard_commitment: Option<[u8; 32]>,
    pub shard_map: Option<FilteredShardMap>,
    pub response_time_us: u64,
    /// BLAKE3(challenge_nonce || chain_head_hash)
    pub challenge_response: [u8; 32],
}

/// Binary verification result -- consistent or not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoSPingResult {
    pub target_position: MatrixPosition,
    pub consistent: bool,
    pub shards_checked: u16,
    pub shards_passed: u16,
    pub response_time_us: u64,
    pub epoch: u64,
}

/// Configuration for the PoSPing verification protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Epoch duration in seconds (default 60).
    pub epoch_duration_secs: u64,
    /// Number of probes per epoch (default 3).
    pub probes_per_epoch: u8,
    /// How many shards to sample per probe (default 5 of 14).
    pub shard_sample_count: u8,
    /// Response timeout in milliseconds (default 5000).
    pub response_timeout_ms: u64,
    /// Consecutive failures before ByzantineViolation (default 3).
    pub max_inconsistency_streak: u8,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            epoch_duration_secs: 60,
            probes_per_epoch: 3,
            shard_sample_count: 5,
            response_timeout_ms: 5000,
            max_inconsistency_streak: 3,
        }
    }
}

/// Privacy-filtered shard map served to verifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredShardMap {
    pub entries: Vec<FilteredShardEntry>,
    /// The commitment hash so verifier can recompute and check.
    pub commitment: [u8; 32],
}

/// Single entry in a privacy-filtered shard map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredShardEntry {
    pub shard_index: u16,
    pub is_parity: bool,
    pub target_position: MatrixPosition,
    pub shard_hash: [u8; 32],
    /// Some if target is tracked, None if untracked.
    pub target_node_id: Option<NodeId>,
}

/// Serde helper for SystemTime (serialize as duration since UNIX_EPOCH).
pub(crate) mod system_time_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}
