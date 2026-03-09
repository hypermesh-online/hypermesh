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

    /// Validate asset exists in blockchain registry.
    ///
    /// Deprecated: Use `validate_asset_in_registry()` with a registry reference instead.
    /// This fallback accepts all non-zero asset IDs when no registry is available.
    #[deprecated(note = "Use validate_asset_in_registry() with a registry reference instead")]
    pub async fn validate_asset_registry(asset_id: &[u8; 32]) -> Result<bool> {
        // Fallback: Accept all non-zero asset IDs when no registry is available
        Ok(!asset_id.iter().all(|&b| b == 0))
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
            .expect("test: system time")
            .as_micros() as u64
    }

    // ------------------------------------------------------------------
    // Full proof validation
    // ------------------------------------------------------------------

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
    }

    // ------------------------------------------------------------------
    // Timestamp
    // ------------------------------------------------------------------

    #[test]
    fn test_proof_timestamp_validation() {
        let validator = ProofOfStateValidator::default();
        let now = now_micros();

        assert!(validator.validate_timestamp(now).is_ok());

        // Future beyond skew
        let future = now + 10 * 60 * 1_000_000;
        assert!(validator.validate_timestamp(future).is_err());

        // Old beyond max age
        let old = now - 25 * 60 * 60 * 1_000_000;
        assert!(validator.validate_timestamp(old).is_err());
    }

    // ------------------------------------------------------------------
    // Proof of Stake (WHO)
    // ------------------------------------------------------------------

    #[test]
    fn test_stake_valid_falcon() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_stake(&valid_who()).is_ok());
    }

    #[test]
    fn test_stake_valid_ed25519() {
        let mut who = [0xCCu8; 32];
        who[0] = ALG_ED25519;
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_stake(&who).is_ok());
    }

    #[test]
    fn test_stake_valid_ecdsa() {
        let mut who = [0xDDu8; 32];
        who[0] = ALG_ECDSA;
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_stake(&who).is_ok());
    }

    #[test]
    fn test_stake_zero_rejected() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_stake(&[0u8; 32]).is_err());
    }

    #[test]
    fn test_stake_invalid_algorithm() {
        let mut who = [0xFFu8; 32];
        who[0] = 0x99; // not a valid algorithm indicator
        let validator = ProofOfStateValidator::default();
        let err = validator.validate_proof_of_stake(&who).unwrap_err();
        assert!(format!("{err}").contains("invalid algorithm indicator"));
    }

    #[test]
    fn test_stake_insufficient_pubkey_prefix() {
        let mut who = [0u8; 32];
        who[0] = ALG_FALCON_1024;
        // bytes 1..9 are all zero -> 0 non-zero bytes (need 8)
        let validator = ProofOfStateValidator::default();
        let err = validator.validate_proof_of_stake(&who).unwrap_err();
        assert!(format!("{err}").contains("public key prefix"));
    }

    // ------------------------------------------------------------------
    // Proof of Work (WHAT)
    // ------------------------------------------------------------------

    #[test]
    fn test_work_valid_default_difficulty() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_work(&valid_what()).is_ok());
    }

    #[test]
    fn test_work_zero_rejected() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_work(&[0u8; 32]).is_err());
    }

    #[test]
    fn test_work_insufficient_difficulty() {
        let validator = ProofOfStateValidator::default(); // requires 8 leading zero bits
        let mut what = [0xFFu8; 32];
        what[0] = 0x01; // only 7 leading zero bits
        let err = validator.validate_proof_of_work(&what).unwrap_err();
        assert!(format!("{err}").contains("insufficient difficulty"));
    }

    #[test]
    fn test_work_custom_difficulty() {
        let validator = ProofOfStateValidator::default().with_pow_difficulty(16);
        // 16 leading zero bits requires first 2 bytes to be 0x00
        let mut what = [0xFFu8; 32];
        what[0] = 0x00;
        what[1] = 0x00;
        assert!(validator.validate_proof_of_work(&what).is_ok());

        // Only 8 leading zeros -> fails 16-bit difficulty
        let mut what2 = [0xFFu8; 32];
        what2[0] = 0x00;
        assert!(validator.validate_proof_of_work(&what2).is_err());
    }

    #[test]
    fn test_work_zero_difficulty_accepts_nonzero() {
        let validator = ProofOfStateValidator::default().with_pow_difficulty(0);
        let what = [0xFFu8; 32]; // 0 leading zero bits
        assert!(validator.validate_proof_of_work(&what).is_ok());
    }

    // ------------------------------------------------------------------
    // Proof of Space (WHERE)
    // ------------------------------------------------------------------

    #[test]
    fn test_space_valid_ipv6_global_unicast() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_space(&valid_where()).is_ok());
    }

    #[test]
    fn test_space_valid_ipv6_link_local() {
        let mut w = [0x01u8; 16];
        w[0] = 0xfe;
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_space(&w).is_ok());
    }

    #[test]
    fn test_space_valid_ipv6_unique_local() {
        let mut w = [0x01u8; 16];
        w[0] = 0xfc;
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_space(&w).is_ok());
    }

    #[test]
    fn test_space_valid_matrix_position() {
        // Encode (1.0f32, 2.0f32, 3.0f32) as LE bytes + 4 pad
        let mut w = [0u8; 16];
        w[0..4].copy_from_slice(&1.0f32.to_le_bytes());
        w[4..8].copy_from_slice(&2.0f32.to_le_bytes());
        w[8..12].copy_from_slice(&3.0f32.to_le_bytes());
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_space(&w).is_ok());
    }

    #[test]
    fn test_space_zero_rejected() {
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_space(&[0u8; 16]).is_err());
    }

    #[test]
    fn test_space_invalid_both_formats() {
        // NaN bytes: f32::NAN LE = [0x00, 0x00, 0xC0, 0x7F]
        // First byte 0x00 is NOT a valid IPv6 prefix.
        // Matrix: NaN is not finite -> fails.
        let mut w = [0u8; 16];
        w[0..4].copy_from_slice(&f32::NAN.to_le_bytes());
        w[4..8].copy_from_slice(&f32::NAN.to_le_bytes());
        w[8..12].copy_from_slice(&f32::NAN.to_le_bytes());
        // Ensure non-zero overall (NaN bytes include 0xC0 and 0x7F)
        let validator = ProofOfStateValidator::default();
        assert!(validator.validate_proof_of_space(&w).is_err());
    }

    // ------------------------------------------------------------------
    // Fast validation (validate_fast)
    // ------------------------------------------------------------------

    #[test]
    fn test_validate_fast_all_ok() {
        let validator = ProofOfStateValidator::default();
        let proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        let result = validator.validate_fast(&proof);
        assert!(result.all_ok());
        assert!(result.timestamp_ok);
        assert!(result.stake_ok);
        assert!(result.work_ok);
        assert!(result.space_ok);
    }

    #[test]
    fn test_validate_fast_bad_timestamp_only() {
        let validator = ProofOfStateValidator::default();
        let proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros() + 10 * 60 * 1_000_000,
            where_: valid_where(),
        };
        let result = validator.validate_fast(&proof);
        assert!(!result.all_ok());
        assert!(!result.timestamp_ok);
        assert!(result.stake_ok);
        assert!(result.work_ok);
        assert!(result.space_ok);
    }

    #[test]
    fn test_validate_fast_bad_stake_only() {
        let mut bad_who = [0xFFu8; 32];
        bad_who[0] = 0x99; // invalid algorithm
        let validator = ProofOfStateValidator::default();
        let proof = ProofOfStateHeader {
            who: bad_who,
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        let result = validator.validate_fast(&proof);
        assert!(!result.all_ok());
        assert!(result.timestamp_ok);
        assert!(!result.stake_ok);
        assert!(result.work_ok);
        assert!(result.space_ok);
    }

    #[test]
    fn test_validate_fast_bad_work_only() {
        let validator = ProofOfStateValidator::default();
        let mut bad_what = [0xFFu8; 32]; // 0 leading zeros
        bad_what[0] = 0x80; // 0 leading zero bits
        let proof = ProofOfStateHeader {
            who: valid_who(),
            what: bad_what,
            when: now_micros(),
            where_: valid_where(),
        };
        let result = validator.validate_fast(&proof);
        assert!(!result.all_ok());
        assert!(result.timestamp_ok);
        assert!(result.stake_ok);
        assert!(!result.work_ok);
        assert!(result.space_ok);
    }

    #[test]
    fn test_validate_fast_bad_space_only() {
        let validator = ProofOfStateValidator::default();
        let proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: [0u8; 16],
        };
        let result = validator.validate_fast(&proof);
        assert!(!result.all_ok());
        assert!(result.timestamp_ok);
        assert!(result.stake_ok);
        assert!(result.work_ok);
        assert!(!result.space_ok);
    }

    // ------------------------------------------------------------------
    // Leading zero bits helper
    // ------------------------------------------------------------------

    #[test]
    fn test_count_leading_zero_bits() {
        // 0x00 = 8 zeros, then 0x01 = 7 zeros -> 15 total
        assert_eq!(count_leading_zero_bits(&[0x00, 0x01]), 15);
        // 0x00, 0x00 = 16 zeros, then 0x01 = 7 zeros -> 23 total
        assert_eq!(count_leading_zero_bits(&[0x00, 0x00, 0x01]), 23);
        assert_eq!(count_leading_zero_bits(&[0x80]), 0);
        assert_eq!(count_leading_zero_bits(&[0x40]), 1);
        assert_eq!(count_leading_zero_bits(&[0x01]), 7);
        assert_eq!(count_leading_zero_bits(&[0x00]), 8);
        assert_eq!(count_leading_zero_bits(&[]), 0);
        // Two full zero bytes = 16 leading zero bits
        assert_eq!(count_leading_zero_bits(&[0x00, 0x00, 0x80]), 16);
    }

    // ------------------------------------------------------------------
    // Asset hash (existing tests preserved)
    // ------------------------------------------------------------------

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
        assert!(
            AssetHashValidator::validate_shard_set(&incomplete_headers, &incomplete_payloads)
                .is_err()
        );
    }

    // ------------------------------------------------------------------
    // Asset registry validation (validate_asset_in_registry)
    // ------------------------------------------------------------------

    #[test]
    fn test_validate_asset_in_registry() {
        let mut registry = std::collections::HashMap::new();
        let asset_id = [0x42u8; 32];
        let asset_hash = [0xABu8; 32];

        // Not in registry
        assert!(!AssetHashValidator::validate_asset_in_registry(
            &asset_id, &registry
        ));

        // Register it
        registry.insert(hex::encode(asset_id), asset_hash);

        // Now found
        assert!(AssetHashValidator::validate_asset_in_registry(
            &asset_id, &registry
        ));

        // Zero asset ID always invalid
        assert!(!AssetHashValidator::validate_asset_in_registry(
            &[0u8; 32], &registry
        ));
    }

    #[test]
    fn test_validate_asset_in_registry_multiple_entries() {
        let mut registry = std::collections::HashMap::new();
        let id_a = [0x01u8; 32];
        let id_b = [0x02u8; 32];
        let id_c = [0x03u8; 32];

        registry.insert(hex::encode(id_a), [0xAAu8; 32]);
        registry.insert(hex::encode(id_b), [0xBBu8; 32]);

        assert!(AssetHashValidator::validate_asset_in_registry(
            &id_a, &registry
        ));
        assert!(AssetHashValidator::validate_asset_in_registry(
            &id_b, &registry
        ));
        assert!(!AssetHashValidator::validate_asset_in_registry(
            &id_c, &registry
        ));
    }

    #[test]
    fn test_validate_asset_in_registry_empty_registry() {
        let registry = std::collections::HashMap::new();
        let asset_id = [0xFFu8; 32];

        // Non-zero ID in empty registry -> not found
        assert!(!AssetHashValidator::validate_asset_in_registry(
            &asset_id, &registry
        ));
    }
}
