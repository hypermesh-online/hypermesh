// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Consistency checking for PoSPing responses.
//!
//! The ConsistencyChecker verifies:
//! 1. Challenge response matches BLAKE3(nonce || chain_head)
//! 2. Shard commitment in response matches recomputed commitment from shard map
//! 3. Response was within timeout
//! Result is binary: consistent (true) or inconsistent (false).

use super::shard_commitment::verify_commitment;
use super::types::{PoSPingRequest, PoSPingResponse, PoSPingResult, VerificationConfig};

/// Verifies PoSPing responses for spatial consistency.
pub struct ConsistencyChecker {
    config: VerificationConfig,
}

impl ConsistencyChecker {
    pub fn new(config: VerificationConfig) -> Self {
        Self { config }
    }

    /// Check a response against its request. Returns a binary result.
    pub fn check(
        &self,
        request: &PoSPingRequest,
        response: &PoSPingResponse,
    ) -> PoSPingResult {
        let mut shards_checked: u16 = 0;
        let mut shards_passed: u16 = 0;
        let mut consistent = true;

        // 1. Verify challenge response: BLAKE3(nonce || chain_head_hash)
        if !self.verify_challenge(request, response) {
            consistent = false;
        }

        // 2. Verify shard commitment matches shard map
        if let (Some(commitment), Some(map)) =
            (&response.shard_commitment, &response.shard_map)
        {
            // Check commitment in response matches map's commitment
            if map.commitment != *commitment {
                consistent = false;
            }
            // Recompute commitment from entries
            if !verify_commitment(map) {
                consistent = false;
            }
            // Count shard checks (sample up to shard_sample_count)
            let sample_count = self
                .config
                .shard_sample_count
                .min(map.entries.len() as u8);
            shards_checked = sample_count as u16;
            // In a real implementation, we'd query shard holders here.
            // For now, if the commitment verifies, all sampled shards pass.
            if consistent {
                shards_passed = shards_checked;
            }
        }

        // 3. Check response time against timeout
        if response.response_time_us > self.config.response_timeout_ms * 1000 {
            consistent = false;
        }

        PoSPingResult {
            target_position: request.target_position,
            consistent,
            shards_checked,
            shards_passed,
            response_time_us: response.response_time_us,
            epoch: request.epoch,
        }
    }

    /// Verify the challenge response: BLAKE3(nonce || chain_head_hash).
    fn verify_challenge(
        &self,
        request: &PoSPingRequest,
        response: &PoSPingResponse,
    ) -> bool {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&request.challenge_nonce);
        hasher.update(&response.chain_head_hash);
        let expected = *hasher.finalize().as_bytes();
        expected == response.challenge_response
    }
}

/// Compute a challenge response for the given nonce and chain head.
/// Used by nodes responding to PoSPing requests.
pub fn compute_challenge_response(nonce: &[u8; 16], chain_head_hash: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(nonce);
    hasher.update(chain_head_hash);
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::shard_commitment::{ShardDistributionMap, ShardPlacement};
    use crate::verification::types::VerificationConfig;
    use hypermesh_lib::{MatrixPosition, NodeId};
    use std::time::SystemTime;

    fn pos(x: f64, y: f64, z: f64) -> MatrixPosition {
        MatrixPosition { x, y, z }
    }

    fn make_request() -> PoSPingRequest {
        PoSPingRequest {
            prober_position: pos(0.0, 0.0, 0.0),
            target_position: pos(10.0, 20.0, 30.0),
            epoch: 1,
            challenge_nonce: [0x42; 16],
        }
    }

    fn make_shard_map() -> (ShardDistributionMap, [u8; 32]) {
        let map = ShardDistributionMap {
            block_index: 1,
            entries: vec![
                ShardPlacement {
                    shard_index: 0,
                    is_parity: false,
                    target_position: pos(1.0, 2.0, 3.0),
                    shard_hash: [0xAA; 32],
                    target_node_id: NodeId::from("node-1"),
                },
                ShardPlacement {
                    shard_index: 1,
                    is_parity: false,
                    target_position: pos(4.0, 5.0, 6.0),
                    shard_hash: [0xBB; 32],
                    target_node_id: NodeId::from("node-2"),
                },
            ],
            created_at: SystemTime::now(),
        };
        let commitment = map.compute_commitment();
        (map, commitment)
    }

    fn make_valid_response(request: &PoSPingRequest) -> PoSPingResponse {
        let chain_head = [0x01; 32];
        let (map, commitment) = make_shard_map();
        let filtered = map.to_filtered_map(true);
        let challenge_response =
            compute_challenge_response(&request.challenge_nonce, &chain_head);

        PoSPingResponse {
            chain_head_hash: chain_head,
            chain_height: 100,
            shard_commitment: Some(commitment),
            shard_map: Some(filtered),
            response_time_us: 1000,
            challenge_response,
        }
    }

    #[test]
    fn valid_response_is_consistent() {
        let checker = ConsistencyChecker::new(VerificationConfig::default());
        let request = make_request();
        let response = make_valid_response(&request);
        let result = checker.check(&request, &response);
        assert!(result.consistent, "valid response must be consistent");
        assert!(result.shards_checked > 0);
        assert_eq!(result.shards_checked, result.shards_passed);
    }

    #[test]
    fn bad_challenge_response_is_inconsistent() {
        let checker = ConsistencyChecker::new(VerificationConfig::default());
        let request = make_request();
        let mut response = make_valid_response(&request);
        response.challenge_response = [0xFF; 32]; // tamper
        let result = checker.check(&request, &response);
        assert!(
            !result.consistent,
            "bad challenge response must be inconsistent"
        );
    }

    #[test]
    fn tampered_shard_map_is_inconsistent() {
        let checker = ConsistencyChecker::new(VerificationConfig::default());
        let request = make_request();
        let mut response = make_valid_response(&request);
        if let Some(map) = response.shard_map.as_mut() {
            map.entries[0].shard_hash = [0xFF; 32]; // tamper
        }
        let result = checker.check(&request, &response);
        assert!(
            !result.consistent,
            "tampered shard map must be inconsistent"
        );
    }

    #[test]
    fn mismatched_commitment_is_inconsistent() {
        let checker = ConsistencyChecker::new(VerificationConfig::default());
        let request = make_request();
        let mut response = make_valid_response(&request);
        response.shard_commitment = Some([0xFF; 32]); // doesn't match map
        let result = checker.check(&request, &response);
        assert!(
            !result.consistent,
            "mismatched commitment must be inconsistent"
        );
    }

    #[test]
    fn timeout_is_inconsistent() {
        let mut config = VerificationConfig::default();
        config.response_timeout_ms = 1; // 1ms timeout
        let checker = ConsistencyChecker::new(config);
        let request = make_request();
        let mut response = make_valid_response(&request);
        response.response_time_us = 5000; // 5ms > 1ms timeout
        let result = checker.check(&request, &response);
        assert!(!result.consistent, "timeout must be inconsistent");
    }

    #[test]
    fn no_shard_data_still_checks_challenge() {
        let checker = ConsistencyChecker::new(VerificationConfig::default());
        let request = make_request();
        let chain_head = [0x01; 32];
        let challenge_response =
            compute_challenge_response(&request.challenge_nonce, &chain_head);
        let response = PoSPingResponse {
            chain_head_hash: chain_head,
            chain_height: 50,
            shard_commitment: None,
            shard_map: None,
            response_time_us: 500,
            challenge_response,
        };
        let result = checker.check(&request, &response);
        assert!(
            result.consistent,
            "no shard data + valid challenge = consistent"
        );
        assert_eq!(result.shards_checked, 0);
    }

    #[test]
    fn compute_challenge_response_is_deterministic() {
        let nonce = [0x42; 16];
        let head = [0x01; 32];
        let r1 = compute_challenge_response(&nonce, &head);
        let r2 = compute_challenge_response(&nonce, &head);
        assert_eq!(r1, r2);
    }
}
