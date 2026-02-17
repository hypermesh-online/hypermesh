// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Validation Logic
//!
//! Implements validation for Proof of State, Asset Hashes, and other
//! HyperMesh intelligence components.

use anyhow::{Result, anyhow};
use blake3::Hasher;
use crate::hypermesh_headers::*;

/// Proof of State validator
pub struct ProofOfStateValidator {
    /// Maximum allowed clock skew (microseconds)
    max_clock_skew: u64,
    /// Maximum proof age (microseconds)
    max_proof_age: u64,
}

impl Default for ProofOfStateValidator {
    fn default() -> Self {
        Self {
            max_clock_skew: 5 * 60 * 1_000_000, // 5 minutes
            max_proof_age: 24 * 60 * 60 * 1_000_000, // 24 hours
        }
    }
}

impl ProofOfStateValidator {
    /// Create new validator with custom settings
    pub fn new(max_clock_skew_secs: u64, max_proof_age_secs: u64) -> Self {
        Self {
            max_clock_skew: max_clock_skew_secs * 1_000_000,
            max_proof_age: max_proof_age_secs * 1_000_000,
        }
    }

    /// Validate Proof of State header
    pub fn validate(&self, proof: &ProofOfStateHeader) -> Result<()> {
        // Validate timestamp
        self.validate_timestamp(proof.when)?;

        // Validate Proof of Stake (WHO)
        self.validate_proof_of_stake(&proof.who)?;

        // Validate Proof of Work (WHAT)
        self.validate_proof_of_work(&proof.what)?;

        // Validate Proof of Space (WHERE)
        self.validate_proof_of_space(&proof.where_)?;

        Ok(())
    }

    /// Validate timestamp with clock skew tolerance
    fn validate_timestamp(&self, timestamp: u64) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_micros() as u64;

        // Check if proof is from the future
        if timestamp > now + self.max_clock_skew {
            return Err(anyhow!(
                "Proof timestamp {} µs in the future (clock skew tolerance: {} µs)",
                timestamp - now,
                self.max_clock_skew
            ));
        }

        // Check if proof is too old
        if now.saturating_sub(timestamp) > self.max_proof_age {
            return Err(anyhow!(
                "Proof timestamp too old: {} µs (max age: {} µs)",
                now - timestamp,
                self.max_proof_age
            ));
        }

        Ok(())
    }

    /// Validate Proof of Stake (WHO) - Identity verification
    fn validate_proof_of_stake(&self, who: &[u8; 32]) -> Result<()> {
        // In production, this would:
        // 1. Verify signature against known public keys
        // 2. Check stake amount in blockchain
        // 3. Validate economic commitment

        // Placeholder: Ensure non-zero
        if who.iter().all(|&b| b == 0) {
            return Err(anyhow!("Proof of Stake is zero"));
        }

        Ok(())
    }

    /// Validate Proof of Work (WHAT) - Computational commitment
    fn validate_proof_of_work(&self, what: &[u8; 32]) -> Result<()> {
        // In production, this would:
        // 1. Verify computational challenge solution
        // 2. Check difficulty meets requirements
        // 3. Validate work was performed

        // Placeholder: Check for valid hash pattern
        // Real PoW would check leading zeros or specific patterns
        if what.iter().all(|&b| b == 0) {
            return Err(anyhow!("Proof of Work is zero"));
        }

        Ok(())
    }

    /// Validate Proof of Space (WHERE) - Storage/location commitment
    fn validate_proof_of_space(&self, where_: &[u8; 16]) -> Result<()> {
        // In production, this would:
        // 1. Verify storage commitment via blockchain
        // 2. Validate matrix position is registered
        // 3. Check location proof is recent

        // Placeholder: Ensure non-zero and valid IPv6
        if where_.iter().all(|&b| b == 0) {
            return Err(anyhow!("Proof of Space is zero"));
        }

        Ok(())
    }

    /// Validate complete four-proof consensus
    pub fn validate_consensus(&self, proof: &ProofOfStateHeader) -> Result<()> {
        // All four proofs must be valid
        self.validate(proof)?;

        // In production, additional consensus checks:
        // - Proof coherence (all proofs from same node)
        // - Blockchain confirmation
        // - Byzantine fault tolerance checks

        tracing::debug!("Four-proof consensus validated for timestamp {}", proof.when);
        Ok(())
    }
}

/// Asset Hash validator
pub struct AssetHashValidator;

impl AssetHashValidator {
    /// Validate asset hash matches payload
    pub fn validate(header: &AssetHashHeader, payload: &[u8]) -> Result<()> {
        // Validate shard indices
        if !header.validate_shard_indices() {
            return Err(anyhow!(
                "Invalid shard indices: {}/{}",
                header.shard_index,
                header.shard_count
            ));
        }

        // Compute BLAKE3 hash of payload
        let computed_hash = Self::compute_hash(payload);

        // Compare with header hash
        if computed_hash != header.hash {
            return Err(anyhow!(
                "Asset hash mismatch: computed {} != header {}",
                hex::encode(computed_hash),
                hex::encode(header.hash)
            ));
        }

        tracing::debug!(
            "Asset hash validated: {} (shard {}/{})",
            hex::encode(header.asset_id),
            header.shard_index,
            header.shard_count
        );

        Ok(())
    }

    /// Compute BLAKE3 hash of data
    pub fn compute_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Validate asset exists in blockchain registry
    pub async fn validate_asset_registry(asset_id: &[u8; 32]) -> Result<bool> {
        // In production, this would:
        // 1. Query blockchain for asset registration
        // 2. Verify asset metadata
        // 3. Check ownership and permissions

        // Placeholder: Accept all non-zero asset IDs
        Ok(!asset_id.iter().all(|&b| b == 0))
    }

    /// Validate complete shard set for multi-part asset
    pub fn validate_shard_set(
        headers: &[AssetHashHeader],
        payloads: &[Vec<u8>],
    ) -> Result<()> {
        if headers.len() != payloads.len() {
            return Err(anyhow!(
                "Header/payload count mismatch: {} != {}",
                headers.len(),
                payloads.len()
            ));
        }

        if headers.is_empty() {
            return Err(anyhow!("Empty shard set"));
        }

        // Verify all shards have same asset_id and shard_count
        let first = &headers[0];
        for header in headers.iter().skip(1) {
            if header.asset_id != first.asset_id {
                return Err(anyhow!("Shard asset_id mismatch"));
            }
            if header.shard_count != first.shard_count {
                return Err(anyhow!("Shard count mismatch"));
            }
        }

        // Validate each shard
        for (header, payload) in headers.iter().zip(payloads.iter()) {
            Self::validate(header, payload)?;
        }

        // Verify we have all shards (no duplicates, no gaps)
        let mut shard_indices: Vec<u32> = headers.iter().map(|h| h.shard_index).collect();
        shard_indices.sort_unstable();

        for (i, &index) in shard_indices.iter().enumerate() {
            if index != i as u32 {
                return Err(anyhow!("Missing shard index {}", i));
            }
        }

        if shard_indices.len() != first.shard_count as usize {
            return Err(anyhow!(
                "Incomplete shard set: {} of {}",
                shard_indices.len(),
                first.shard_count
            ));
        }

        tracing::info!(
            "Validated complete shard set for asset {}: {} shards",
            hex::encode(first.asset_id),
            first.shard_count
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_of_state_validation() {
        let validator = ProofOfStateValidator::default();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let valid_proof = ProofOfStateHeader {
            who: [1u8; 32],
            what: [2u8; 32],
            when: now,
            where_: [3u8; 16],
        };

        assert!(validator.validate(&valid_proof).is_ok());
    }

    #[test]
    fn test_proof_timestamp_validation() {
        let validator = ProofOfStateValidator::default();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        // Valid timestamp
        assert!(validator.validate_timestamp(now).is_ok());

        // Future timestamp (beyond skew)
        let future = now + 10 * 60 * 1_000_000; // 10 minutes
        assert!(validator.validate_timestamp(future).is_err());

        // Old timestamp (beyond max age)
        let old = now - 25 * 60 * 60 * 1_000_000; // 25 hours
        assert!(validator.validate_timestamp(old).is_err());
    }

    #[test]
    fn test_asset_hash_validation() {
        let payload = b"test data for hashing";
        let hash = AssetHashValidator::compute_hash(payload);

        let header = AssetHashHeader {
            asset_id: [1u8; 32],
            hash,
            shard_count: 1,
            shard_index: 0,
        };

        assert!(AssetHashValidator::validate(&header, payload).is_ok());

        // Wrong hash
        let mut bad_header = header.clone();
        bad_header.hash = [0u8; 32];
        assert!(AssetHashValidator::validate(&bad_header, payload).is_err());

        // Invalid shard index
        let mut bad_header = header.clone();
        bad_header.shard_index = 5;
        assert!(AssetHashValidator::validate(&bad_header, payload).is_err());
    }

    #[test]
    fn test_shard_set_validation() {
        let payloads = vec![
            b"shard 0 data".to_vec(),
            b"shard 1 data".to_vec(),
            b"shard 2 data".to_vec(),
        ];

        let headers: Vec<AssetHashHeader> = payloads
            .iter()
            .enumerate()
            .map(|(i, payload)| AssetHashHeader {
                asset_id: [1u8; 32],
                hash: AssetHashValidator::compute_hash(payload),
                shard_count: 3,
                shard_index: i as u32,
            })
            .collect();

        assert!(AssetHashValidator::validate_shard_set(&headers, &payloads).is_ok());

        // Missing shard
        let incomplete_headers = headers[0..2].to_vec();
        let incomplete_payloads = payloads[0..2].to_vec();
        assert!(AssetHashValidator::validate_shard_set(&incomplete_headers, &incomplete_payloads).is_err());
    }
}
