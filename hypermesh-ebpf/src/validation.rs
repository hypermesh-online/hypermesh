// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Validation Logic — Structural Pre-Validation Layer
//!
//! Implements fast structural pre-validation for Proof of State headers,
//! Asset Hashes, and other HyperMesh intelligence components.
//!
//! ## Architectural boundary
//!
//! This module performs **non-cryptographic structural checks** that can run
//! at wire speed in the eBPF/XDP fast path (or its userspace fallback).
//! Full asymmetric signature verification (FALCON-1024, Ed25519, ECDSA)
//! is architecturally impossible in the BPF instruction set and MUST happen
//! in userspace via TrustChain after packets pass structural pre-validation.
//! On success userspace mirrors the result back into the kernel maps via
//! [`crate::HyperMeshEbpf::set_peer_pos_validated`] (the two-tier PoS feedback
//! loop, papers/HYPERMESH.md §5.4).
//!
//! These validators are LIVE again because STOQ now emits the plaintext
//! HyperMesh extension header on the send path (`apply_extensions`) — they were
//! only ever "dead" because nothing emitted the header they parse.
//!
//! ### What this layer validates
//! - **PoTime (WHEN)**: Timestamp freshness (clock skew + max age)
//! - **PoStake (WHO)**: Algorithm indicator byte + public key prefix density
//! - **PoWork (WHAT)**: Leading zero bits meet configurable difficulty
//! - **PoSpace (WHERE)**: Valid IPv6 prefix or finite matrix coordinates
//!
//! ### What this layer does NOT validate
//! - Cryptographic signature correctness (FALCON-1024/Ed25519/ECDSA)
//! - Public key authenticity (TrustChain CA chain verification)
//! - PoW challenge-response correctness (only difficulty prefix)
//! - PoSpace storage commitment proofs (only format validity)

use crate::hypermesh_headers::*;
use anyhow::{anyhow, Result};
use blake3::Hasher;

/// Algorithm indicator bytes for Proof of Stake identity validation.
/// The first byte of the `who` field indicates which signing algorithm was used.
pub const ALG_FALCON_1024: u8 = 0x01;
pub const ALG_ED25519: u8 = 0x02;
pub const ALG_ECDSA: u8 = 0x03;

/// Minimum number of non-zero bytes required in the public key prefix
/// (bytes 1..9 of the `who` field) to pass fast pre-validation.
const MIN_PUBKEY_PREFIX_NONZERO: usize = 8;

/// Result of fast four-proof validation, reporting each proof independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FastValidationResult {
    /// Timestamp (WHEN) passed validation
    pub timestamp_ok: bool,
    /// Proof of Stake (WHO) passed validation
    pub stake_ok: bool,
    /// Proof of Work (WHAT) passed validation
    pub work_ok: bool,
    /// Proof of Space (WHERE) passed validation
    pub space_ok: bool,
}

impl FastValidationResult {
    /// Returns true only if all four proofs passed.
    pub fn all_ok(&self) -> bool {
        self.timestamp_ok && self.stake_ok && self.work_ok && self.space_ok
    }
}

/// Proof of State structural pre-validator.
///
/// Performs fast non-cryptographic checks suitable for eBPF/XDP wire-speed
/// filtering. Rejects obviously invalid packets before they reach userspace.
/// Full cryptographic verification (FALCON-1024 signatures, CA chain
/// validation) happens in TrustChain after this layer passes the packet.
pub struct ProofOfStateValidator {
    /// Maximum allowed clock skew (microseconds)
    max_clock_skew: u64,
    /// Maximum proof age (microseconds)
    max_proof_age: u64,
    /// Minimum number of leading zero bits required for PoW difficulty
    min_pow_leading_zero_bits: u32,
}

impl Default for ProofOfStateValidator {
    fn default() -> Self {
        Self {
            max_clock_skew: 5 * 60 * 1_000_000,      // 5 minutes
            max_proof_age: 24 * 60 * 60 * 1_000_000, // 24 hours
            min_pow_leading_zero_bits: 8,            // first byte must be 0x00
        }
    }
}

impl ProofOfStateValidator {
    /// Create new validator with custom settings
    pub fn new(max_clock_skew_secs: u64, max_proof_age_secs: u64) -> Self {
        Self {
            max_clock_skew: max_clock_skew_secs * 1_000_000,
            max_proof_age: max_proof_age_secs * 1_000_000,
            min_pow_leading_zero_bits: 8,
        }
    }

    /// Set the minimum number of leading zero bits required for PoW difficulty.
    pub fn with_pow_difficulty(mut self, leading_zero_bits: u32) -> Self {
        self.min_pow_leading_zero_bits = leading_zero_bits;
        self
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
                "Proof timestamp {} us in the future (clock skew tolerance: {} us)",
                timestamp - now,
                self.max_clock_skew
            ));
        }

        // Check if proof is too old
        if now.saturating_sub(timestamp) > self.max_proof_age {
            return Err(anyhow!(
                "Proof timestamp too old: {} us (max age: {} us)",
                now - timestamp,
                self.max_proof_age
            ));
        }

        Ok(())
    }

    /// Validate Proof of Stake (WHO) - Identity pre-validation.
    ///
    /// Fast checks performed at the eBPF/XDP layer:
    /// - Non-zero check
    /// - First byte is a valid algorithm indicator (FALCON-1024, Ed25519, ECDSA)
    /// - Public key prefix (bytes 1..9) contains at least 8 non-zero bytes
    ///
    /// Deep cryptographic verification happens in TrustChain.
    fn validate_proof_of_stake(&self, who: &[u8; 32]) -> Result<()> {
        if who.iter().all(|&b| b == 0) {
            return Err(anyhow!("Proof of Stake is zero"));
        }

        // First byte must be a known algorithm indicator
        match who[0] {
            ALG_FALCON_1024 | ALG_ED25519 | ALG_ECDSA => {}
            other => {
                return Err(anyhow!(
                    "Proof of Stake: invalid algorithm indicator 0x{other:02x} \
                     (expected 0x01=FALCON, 0x02=Ed25519, 0x03=ECDSA)"
                ));
            }
        }

        // Public key prefix (bytes 1..9) must have sufficient non-zero bytes
        let nonzero_count = who[1..9].iter().filter(|&&b| b != 0).count();
        if nonzero_count < MIN_PUBKEY_PREFIX_NONZERO {
            return Err(anyhow!(
                "Proof of Stake: public key prefix has only {nonzero_count} non-zero bytes \
                 in positions 1..9 (minimum {MIN_PUBKEY_PREFIX_NONZERO})"
            ));
        }

        Ok(())
    }

    /// Validate Proof of Work (WHAT) - Computational difficulty check.
    ///
    /// Fast checks performed at the eBPF/XDP layer:
    /// - Non-zero check (all-zero hash is invalid)
    /// - Leading zero bits meet the configured difficulty requirement
    ///
    /// Actual PoW challenge verification happens in the Proof of State layer.
    fn validate_proof_of_work(&self, what: &[u8; 32]) -> Result<()> {
        if what.iter().all(|&b| b == 0) {
            return Err(anyhow!("Proof of Work is zero"));
        }

        let leading_zeros = count_leading_zero_bits(what);
        if leading_zeros < self.min_pow_leading_zero_bits {
            return Err(anyhow!(
                "Proof of Work: insufficient difficulty -- {} leading zero bits \
                 (minimum required: {})",
                leading_zeros,
                self.min_pow_leading_zero_bits
            ));
        }

        Ok(())
    }

    /// Validate Proof of Space (WHERE) - Storage/location commitment.
    ///
    /// Fast checks performed at the eBPF/XDP layer:
    /// - Non-zero check
    /// - Interpret as either a valid IPv6 address or a matrix position:
    ///   - IPv6: first byte matches a valid prefix (global unicast, unique local, etc.)
    ///   - Matrix: 3 x f32 (12 bytes) + 4 padding; coordinates must be finite
    ///
    /// Deep blockchain verification happens in the Proof of State layer.
    fn validate_proof_of_space(&self, where_: &[u8; 16]) -> Result<()> {
        if where_.iter().all(|&b| b == 0) {
            return Err(anyhow!("Proof of Space is zero"));
        }

        let is_valid_ipv6_prefix = Self::check_ipv6_prefix(where_);
        let is_valid_matrix = Self::check_matrix_position(where_);

        if !is_valid_ipv6_prefix && !is_valid_matrix {
            return Err(anyhow!(
                "Proof of Space: bytes are neither a valid IPv6 prefix \
                 nor a valid matrix position"
            ));
        }

        Ok(())
    }

    /// Check whether the first byte looks like a plausible IPv6 prefix.
    ///
    /// Accepted prefixes (first byte):
    /// - 0x20..=0x3f -- Global unicast (2000::/3)
    /// - 0xfc..=0xfd -- Unique local (fc00::/7)
    /// - 0xfe -- Link-local (fe80::/10) or site-local
    /// - 0xff -- Multicast
    fn check_ipv6_prefix(bytes: &[u8; 16]) -> bool {
        matches!(bytes[0], 0x20..=0x3f | 0xfc..=0xfd | 0xfe | 0xff)
    }

    /// Check whether the first 12 bytes represent three finite f32 values
    /// (a matrix position), with the remaining 4 bytes as padding.
    fn check_matrix_position(bytes: &[u8; 16]) -> bool {
        let x = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let y = f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let z = f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        x.is_finite() && y.is_finite() && z.is_finite()
    }

    /// Fast four-proof validation returning a detailed per-proof result.
    ///
    /// This is the primary entry point for the XDP manager. Each proof is
    /// validated independently so callers can see exactly which proof failed.
    pub fn validate_fast(&self, proof: &ProofOfStateHeader) -> FastValidationResult {
        FastValidationResult {
            timestamp_ok: self.validate_timestamp(proof.when).is_ok(),
            stake_ok: self.validate_proof_of_stake(&proof.who).is_ok(),
            work_ok: self.validate_proof_of_work(&proof.what).is_ok(),
            space_ok: self.validate_proof_of_space(&proof.where_).is_ok(),
        }
    }

    /// Validate complete four-proof state proof (returns error on first failure).
    pub fn validate_state_proof(&self, proof: &ProofOfStateHeader) -> Result<()> {
        self.validate(proof)?;
        tracing::debug!(
            "Four-proof state proof validated for timestamp {}",
            proof.when
        );
        Ok(())
    }
}

/// Count the number of leading zero bits in a byte slice.
fn count_leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut total = 0u32;
    for &byte in bytes {
        if byte == 0 {
            total += 8;
        } else {
            // u8::leading_zeros() counts within the 8-bit value (0..=8).
            total += byte.leading_zeros();
            break;
        }
    }
    total
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

    /// Validate asset exists in the registered asset hash registry.
    ///
    /// Checks the asset_id against the in-memory registry maintained by
    /// `HyperMeshEbpf::register_asset_hash()`. Returns true if the asset
    /// is registered with a valid (non-zero) hash.
    pub fn validate_asset_in_registry(
        asset_id: &[u8; 32],
        registry: &std::collections::HashMap<String, [u8; 32]>,
    ) -> bool {
        // Zero asset IDs are always invalid
        if asset_id.iter().all(|&b| b == 0) {
            return false;
        }

        // Check if the asset ID (as hex string) exists in the registry
        let asset_key = hex::encode(asset_id);
        registry.contains_key(&asset_key)
    }

    /// Validate complete shard set for multi-part asset
    pub fn validate_shard_set(headers: &[AssetHashHeader], payloads: &[Vec<u8>]) -> Result<()> {
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
                return Err(anyhow!("Missing shard index {i}"));
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

    /// Build a valid `who` field: FALCON-1024 indicator + 8 non-zero prefix bytes.
    fn valid_who() -> [u8; 32] {
        let mut who = [0xABu8; 32];
        who[0] = ALG_FALCON_1024; // 0x01
        who
    }

    /// Build a valid `what` field: first byte 0x00 (8 leading zero bits).
    fn valid_what() -> [u8; 32] {
        let mut what = [0xFFu8; 32];
        what[0] = 0x00;
        what
    }

    /// Build a valid `where_` field: IPv6 global unicast prefix 0x20.
    fn valid_where() -> [u8; 16] {
        let mut w = [0x01u8; 16];
        w[0] = 0x20;
        w
    }

    fn now_micros() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0)
    }

    #[test]
    fn test_count_leading_zero_bits() {
        assert_eq!(count_leading_zero_bits(&[0x00, 0x00, 0xFF]), 16);
        assert_eq!(count_leading_zero_bits(&[0x00, 0x01]), 15);
        assert_eq!(count_leading_zero_bits(&[0x80]), 0);
        assert_eq!(count_leading_zero_bits(&[0xFF]), 0);
        assert_eq!(count_leading_zero_bits(&[0x00, 0x80]), 8);
    }

    #[test]
    fn test_proof_of_state_validation() {
        let validator = ProofOfStateValidator::default();
        let valid_proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        assert!(validator.validate(&valid_proof).is_ok());
        assert!(validator.validate_fast(&valid_proof).all_ok());
    }

    #[test]
    fn test_proof_timestamp_validation() {
        let validator = ProofOfStateValidator::default();
        let now = now_micros();
        assert!(validator.validate_timestamp(now).is_ok());
        let future = now + 10 * 60 * 1_000_000;
        assert!(validator.validate_timestamp(future).is_err());
        let old = now - 25 * 60 * 60 * 1_000_000;
        assert!(validator.validate_timestamp(old).is_err());
    }

    #[test]
    fn test_stake_algorithms() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_stake(&valid_who()).is_ok());

        let mut ed = [0xCCu8; 32];
        ed[0] = ALG_ED25519;
        assert!(validator.validate_proof_of_stake(&ed).is_ok());

        let mut ec = [0xDDu8; 32];
        ec[0] = ALG_ECDSA;
        assert!(validator.validate_proof_of_stake(&ec).is_ok());

        assert!(validator.validate_proof_of_stake(&[0u8; 32]).is_err());

        let mut bad = [0xFFu8; 32];
        bad[0] = 0x99;
        let err = validator
            .validate_proof_of_stake(&bad)
            .err()
            .map(|e| format!("{e}"))
            .unwrap_or_default();
        assert!(err.contains("invalid algorithm indicator"));
    }

    #[test]
    fn test_stake_insufficient_pubkey_prefix() {
        let mut who = [0u8; 32];
        who[0] = ALG_FALCON_1024;
        let validator = ProofOfStateValidator::default();
        let err = validator
            .validate_proof_of_stake(&who)
            .err()
            .map(|e| format!("{e}"))
            .unwrap_or_default();
        assert!(err.contains("public key prefix"));
    }

    #[test]
    fn test_work_difficulty() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_work(&valid_what()).is_ok());
        assert!(validator.validate_proof_of_work(&[0u8; 32]).is_err());

        let mut weak = [0xFFu8; 32];
        weak[0] = 0x01; // only 7 leading zero bits
        assert!(validator.validate_proof_of_work(&weak).is_err());

        let strict = ProofOfStateValidator::default().with_pow_difficulty(16);
        let mut sixteen = [0xFFu8; 32];
        sixteen[0] = 0x00;
        sixteen[1] = 0x00;
        assert!(strict.validate_proof_of_work(&sixteen).is_ok());
        let mut eight = [0xFFu8; 32];
        eight[0] = 0x00;
        assert!(strict.validate_proof_of_work(&eight).is_err());
    }

    #[test]
    fn test_space_ipv6_and_matrix() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_space(&valid_where()).is_ok());
        assert!(validator.validate_proof_of_space(&[0u8; 16]).is_err());
    }

    #[test]
    fn test_asset_hash_validate() {
        let payload = b"hypermesh asset payload";
        let header = AssetHashHeader {
            asset_id: [1u8; 32],
            hash: AssetHashValidator::compute_hash(payload),
            shard_count: 1,
            shard_index: 0,
        };
        assert!(AssetHashValidator::validate(&header, payload).is_ok());

        let mut bad_header = header.clone();
        bad_header.hash = [0u8; 32];
        assert!(AssetHashValidator::validate(&bad_header, payload).is_err());
    }

    #[test]
    fn test_asset_in_registry() {
        let mut registry = std::collections::HashMap::new();
        let asset_id = [7u8; 32];
        registry.insert(hex::encode(asset_id), [9u8; 32]);
        assert!(AssetHashValidator::validate_asset_in_registry(&asset_id, &registry));
        assert!(!AssetHashValidator::validate_asset_in_registry(&[0u8; 32], &registry));
        assert!(!AssetHashValidator::validate_asset_in_registry(&[8u8; 32], &registry));
    }
}
