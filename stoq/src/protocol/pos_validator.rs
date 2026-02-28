// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof of State Token Validation Module
//!
//! Provides protocol-layer validation for PoS tokens within STOQ transport.
//! Integrates with TrustChain for certificate validation and token verification.

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use dashmap::DashMap;
use tracing::{debug, info};
use serde::{Serialize, Deserialize};

pub use super::falcon_trustchain::{TrustChainClient, FalconTrustChainClient};

/// Backward compatibility alias for old test API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofData<T = ()> {
    pub storage_commitment: Vec<u8>,
    pub location: String,
    pub size_bytes: u64,
    pub content_hash: Vec<u8>,
    pub stake_amount: u64,
    pub owner_pubkey: Vec<u8>,
    pub lock_period_blocks: u64,
    pub delegation_proof: Vec<u8>,
    pub computation_hash: Vec<u8>,
    pub difficulty_target: u64,
    pub resource_type: String,
    pub nonce: u64,
    pub timestamp: SystemTime,
    pub vdf_proof: Vec<u8>,
    pub chain_height: u64,
    pub previous_block: Vec<u8>,
    _phantom: std::marker::PhantomData<T>,
}

impl Default for ProofData {
    fn default() -> Self {
        Self {
            storage_commitment: Vec::new(),
            location: String::new(),
            size_bytes: 0,
            content_hash: Vec::new(),
            stake_amount: 0,
            owner_pubkey: Vec::new(),
            lock_period_blocks: 0,
            delegation_proof: Vec::new(),
            computation_hash: Vec::new(),
            difficulty_target: 0,
            resource_type: String::new(),
            nonce: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            vdf_proof: Vec::new(),
            chain_height: 0,
            previous_block: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Proof of State token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosToken {
    /// Token identifier
    pub id: Vec<u8>,

    /// Proof of Space - WHERE (storage location and commitment)
    pub proof_of_space: ProofOfSpace,

    /// Proof of Stake - WHO (ownership and access rights)
    pub proof_of_stake: ProofOfStake,

    /// Proof of Work - WHAT/HOW (computational resources)
    pub proof_of_work: ProofOfWork,

    /// Proof of Time - WHEN (temporal ordering)
    pub proof_of_time: ProofOfTime,

    /// Token signature (from issuer)
    pub signature: Vec<u8>,

    /// Token expiry time
    pub expires_at: SystemTime,

    /// Backward compatibility: issuer public key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_pubkey: Option<Vec<u8>>,
}

/// Proof of Space component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfSpace {
    /// Storage commitment hash
    pub commitment_hash: Vec<u8>,

    /// Matrix position (x, y, z) in Block-MATRIX topology
    pub matrix_position: (u32, u32, u32),

    /// Storage capacity in bytes
    pub capacity: u64,
}

/// Proof of Stake component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfStake {
    /// Stake owner public key
    pub owner_pubkey: Vec<u8>,

    /// Economic stake amount
    pub stake_amount: u64,

    /// Stake duration
    pub staked_until: SystemTime,
}

/// Proof of Work component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfWork {
    /// Work difficulty target
    pub difficulty: u32,

    /// Work nonce
    pub nonce: u64,

    /// Work hash
    pub work_hash: Vec<u8>,
}

/// Proof of Time component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfTime {
    /// Timestamp of creation
    pub timestamp: SystemTime,

    /// Sequence number for ordering
    pub sequence: u64,

    /// Previous block hash for chain continuity
    pub prev_hash: Vec<u8>,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    /// Is the token valid
    pub is_valid: bool,

    /// Validation errors if any
    pub errors: Vec<String>,

    /// Time taken for validation
    pub validation_time: Duration,
}

/// Token validation cache entry
#[derive(Debug, Clone)]
struct CachedValidation {
    /// Validation result
    result: bool,

    /// Cache expiry time
    expires_at: SystemTime,
}

/// PoS Token Validator with TrustChain integration
pub struct PosTokenValidator {
    /// Cache for validated tokens (5-minute TTL)
    validation_cache: Arc<DashMap<Vec<u8>, CachedValidation>>,

    /// Cache TTL duration
    cache_ttl: Duration,

    /// TrustChain integration (would connect to TrustChain component)
    trustchain_client: Option<Arc<dyn TrustChainClient>>,

    /// Validation metrics
    metrics: Arc<ValidationMetrics>,
}

/// Validation metrics tracking
struct ValidationMetrics {
    /// Total validations performed
    total_validations: std::sync::atomic::AtomicU64,

    /// Cache hits
    cache_hits: std::sync::atomic::AtomicU64,

    /// Cache misses
    cache_misses: std::sync::atomic::AtomicU64,

    /// Failed validations
    failed_validations: std::sync::atomic::AtomicU64,

    /// Average validation time in microseconds
    avg_validation_time_us: std::sync::atomic::AtomicU64,
}

impl PosTokenValidator {
    /// Create a new PoS token validator
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            validation_cache: Arc::new(DashMap::new()),
            cache_ttl,
            trustchain_client: None,
            metrics: Arc::new(ValidationMetrics {
                total_validations: std::sync::atomic::AtomicU64::new(0),
                cache_hits: std::sync::atomic::AtomicU64::new(0),
                cache_misses: std::sync::atomic::AtomicU64::new(0),
                failed_validations: std::sync::atomic::AtomicU64::new(0),
                avg_validation_time_us: std::sync::atomic::AtomicU64::new(0),
            }),
        }
    }

    /// Set TrustChain client for certificate validation
    pub fn set_trustchain_client(&mut self, client: Arc<dyn TrustChainClient>) {
        self.trustchain_client = Some(client);
    }

    /// Validate a PoS token
    pub fn validate_token(&self, token: &PosToken) -> Result<ValidationResult> {
        let start_time = std::time::Instant::now();

        // Update metrics
        self.metrics.total_validations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Check cache first
        if let Some(cached) = self.validation_cache.get(&token.id) {
            if cached.expires_at > SystemTime::now() {
                self.metrics.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                debug!("Token validation cache hit for {:?}", token.id);

                return Ok(ValidationResult {
                    is_valid: cached.result,
                    errors: if cached.result { vec![] } else { vec!["Cached validation failure".to_string()] },
                    validation_time: start_time.elapsed(),
                });
            }
        }

        self.metrics.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut errors = Vec::new();

        // 1. Check token expiry
        if token.expires_at <= SystemTime::now() {
            errors.push("Token has expired".to_string());
        }

        // 2. Validate Proof of Space
        if !self.validate_proof_of_space(&token.proof_of_space) {
            errors.push("Invalid Proof of Space".to_string());
        }

        // 3. Validate Proof of Stake
        if !self.validate_proof_of_stake(&token.proof_of_stake) {
            errors.push("Invalid Proof of Stake".to_string());
        }

        // 4. Validate Proof of Work
        if !self.validate_proof_of_work(&token.proof_of_work) {
            errors.push("Invalid Proof of Work".to_string());
        }

        // 5. Validate Proof of Time
        if !self.validate_proof_of_time(&token.proof_of_time) {
            errors.push("Invalid Proof of Time".to_string());
        }

        // 6. Verify signature if TrustChain is available
        if let Some(ref client) = self.trustchain_client {
            // Serialize token data for signature verification
            let token_data = self.serialize_token_for_signing(token);

            match client.verify_signature(
                &token.proof_of_stake.owner_pubkey,
                &token_data,
                &token.signature
            ) {
                Ok(true) => {
                    debug!("Token signature verified successfully");
                }
                Ok(false) => {
                    errors.push("Invalid token signature".to_string());
                }
                Err(e) => {
                    errors.push(format!("Signature verification failed: {}", e));
                }
            }
        } else {
            // No TrustChain client configured. Wire FalconTrustChainClient
            // via set_trustchain_client() for production FALCON-1024 verification.
            debug!("TrustChain client not configured, skipping signature verification");
        }

        let is_valid = errors.is_empty();

        // Cache the result
        let cached_entry = CachedValidation {
            result: is_valid,
            expires_at: SystemTime::now() + self.cache_ttl,
        };
        self.validation_cache.insert(token.id.clone(), cached_entry);

        // Update metrics
        if !is_valid {
            self.metrics.failed_validations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        let validation_time = start_time.elapsed();
        let validation_us = validation_time.as_micros() as u64;

        // Update average validation time (simple moving average)
        let current_avg = self.metrics.avg_validation_time_us.load(std::sync::atomic::Ordering::Relaxed);
        let new_avg = (current_avg * 9 + validation_us) / 10; // Weighted average
        self.metrics.avg_validation_time_us.store(new_avg, std::sync::atomic::Ordering::Relaxed);

        Ok(ValidationResult {
            is_valid,
            errors,
            validation_time,
        })
    }

    /// Validate Proof of Space component
    fn validate_proof_of_space(&self, pos: &ProofOfSpace) -> bool {
        // Check commitment hash is not empty
        if pos.commitment_hash.is_empty() {
            return false;
        }

        // Check matrix position is valid (non-zero)
        if pos.matrix_position == (0, 0, 0) {
            return false;
        }

        // Check capacity is reasonable (at least 1KB, at most 1PB)
        if pos.capacity < 1024 || pos.capacity > 1024_u64.pow(5) {
            return false;
        }

        true
    }

    /// Validate Proof of Stake component
    fn validate_proof_of_stake(&self, pos: &ProofOfStake) -> bool {
        // Check owner pubkey is not empty
        if pos.owner_pubkey.is_empty() {
            return false;
        }

        // Check stake amount is non-zero
        if pos.stake_amount == 0 {
            return false;
        }

        // Check stake hasn't expired
        if pos.staked_until <= SystemTime::now() {
            return false;
        }

        true
    }

    /// Validate Proof of Work component
    ///
    /// Verifies that the work hash has at least `difficulty` leading zero bits,
    /// matching the same algorithm used by `hypermesh_ebpf::validation::count_leading_zero_bits`.
    fn validate_proof_of_work(&self, pow: &ProofOfWork) -> bool {
        // Check work hash is not empty
        if pow.work_hash.is_empty() {
            return false;
        }

        // Difficulty must be non-zero
        if pow.difficulty == 0 {
            return false;
        }

        // Count leading zero bits in the work hash and verify against difficulty
        let leading_zeros = count_leading_zero_bits(&pow.work_hash);
        if leading_zeros < pow.difficulty {
            debug!(
                "PoW failed: hash has {} leading zero bits, need {}",
                leading_zeros, pow.difficulty
            );
            return false;
        }

        true
    }

    /// Validate Proof of Time component
    fn validate_proof_of_time(&self, pot: &ProofOfTime) -> bool {
        // Check timestamp is not in the future
        if pot.timestamp > SystemTime::now() + Duration::from_secs(60) {
            // Allow 1 minute clock skew
            return false;
        }

        // Check timestamp is not too old (max 24 hours)
        if pot.timestamp < SystemTime::now() - Duration::from_secs(86400) {
            return false;
        }

        // Check previous hash is not empty (except for genesis)
        if pot.sequence > 0 && pot.prev_hash.is_empty() {
            return false;
        }

        true
    }

    /// Serialize token for signature verification using canonical length-prefixed format.
    ///
    /// Each field is encoded as a u32 LE length prefix followed by the field bytes.
    /// This prevents ambiguity from variable-length field concatenation.
    fn serialize_token_for_signing(&self, token: &PosToken) -> Vec<u8> {
        let mut buf = Vec::new();

        // Helper closure: write u32 LE length prefix then field bytes
        let mut write_field = |field: &[u8]| {
            buf.extend_from_slice(&(field.len() as u32).to_le_bytes());
            buf.extend_from_slice(field);
        };

        write_field(&token.id);
        write_field(&token.proof_of_space.commitment_hash);
        write_field(&token.proof_of_stake.owner_pubkey);
        write_field(&token.proof_of_work.work_hash);
        write_field(&token.proof_of_time.prev_hash);

        buf
    }

    /// Clear the validation cache
    pub fn clear_cache(&self) {
        self.validation_cache.clear();
        info!("Validation cache cleared");
    }

    /// Get validation metrics
    pub fn get_metrics(&self) -> ValidationStats {
        ValidationStats {
            total_validations: self.metrics.total_validations.load(std::sync::atomic::Ordering::Relaxed),
            cache_hits: self.metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed),
            cache_misses: self.metrics.cache_misses.load(std::sync::atomic::Ordering::Relaxed),
            failed_validations: self.metrics.failed_validations.load(std::sync::atomic::Ordering::Relaxed),
            avg_validation_time_us: self.metrics.avg_validation_time_us.load(std::sync::atomic::Ordering::Relaxed),
            cache_size: self.validation_cache.len(),
        }
    }
}

/// Count leading zero bits in a byte slice.
///
/// Mirrors the algorithm in `hypermesh_ebpf::validation::count_leading_zero_bits`.
/// A hash with N leading zero bits meets difficulty N.
fn count_leading_zero_bits(data: &[u8]) -> u32 {
    let mut count: u32 = 0;
    for &byte in data {
        if byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros();
            break;
        }
    }
    count
}

/// Validation statistics
#[derive(Debug)]
pub struct ValidationStats {
    pub total_validations: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub failed_validations: u64,
    pub avg_validation_time_us: u64,
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_token() -> PosToken {
        PosToken {
            id: vec![1, 2, 3, 4],
            issuer_pubkey: Some(vec![20, 21, 22, 23]),
            proof_of_space: ProofOfSpace {
                commitment_hash: vec![5, 6, 7, 8],
                matrix_position: (1, 2, 3),
                capacity: 1024 * 1024, // 1MB
            },
            proof_of_stake: ProofOfStake {
                owner_pubkey: vec![9, 10, 11, 12],
                stake_amount: 1000,
                staked_until: SystemTime::now() + Duration::from_secs(3600),
            },
            proof_of_work: ProofOfWork {
                // 2 zero bytes = 16 leading zero bits, meeting difficulty 10
                difficulty: 10,
                nonce: 12345,
                work_hash: vec![0, 0, 0x0F, 0xFF],
            },
            proof_of_time: ProofOfTime {
                timestamp: SystemTime::now(),
                sequence: 1,
                prev_hash: vec![17, 18, 19, 20],
            },
            signature: vec![21, 22, 23, 24],
            expires_at: SystemTime::now() + Duration::from_secs(300),
        }
    }

    #[test]
    fn test_token_validation() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let token = create_test_token();

        let result = validator.validate_token(&token).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_expired_token() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let mut token = create_test_token();
        token.expires_at = SystemTime::now() - Duration::from_secs(60);

        let result = validator.validate_token(&token).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("expired")));
    }

    #[test]
    fn test_cache_functionality() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let token = create_test_token();

        // First validation should miss cache
        let result1 = validator.validate_token(&token).unwrap();
        assert!(result1.is_valid);

        let stats = validator.get_metrics();
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hits, 0);

        // Second validation should hit cache
        let result2 = validator.validate_token(&token).unwrap();
        assert!(result2.is_valid);

        let stats = validator.get_metrics();
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hits, 1);
    }

    #[test]
    fn test_validation_metrics() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let token = create_test_token();

        // Perform multiple validations
        for _ in 0..5 {
            let _ = validator.validate_token(&token);
        }

        let stats = validator.get_metrics();
        assert_eq!(stats.total_validations, 5);
        let _ = stats.avg_validation_time_us;
    }

}