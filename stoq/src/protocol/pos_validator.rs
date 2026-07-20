// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof of State Token Validation Module
//!
//! Provides protocol-layer validation for PoS tokens within STOQ transport.
//! Integrates with TrustChain for certificate validation and token verification.

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

use hypermesh_lib::proof::StateProof;

pub use super::falcon_trustchain::{FalconTrustChainClient, TrustChainClient};

/// Proof of State token carried on the STOQ wire.
///
/// CANONICAL MODEL: the four proofs are **not** redefined here. The single
/// source of truth for `StakeProof` / `WorkProof` / `SpaceProof` / `TimeProof`
/// and their composite `StateProof` is `hypermesh_lib::proof`. This token wraps
/// that canonical proof with the transport-level bindings STOQ needs
/// (matrix position, chain continuity, signature, expiry).
///
/// - **PoStake = WHO / authorization.** `issuer_pubkey` is the FALCON-1024
///   identity binding; `proof.stake_proof.stake_holder_id` is its BLAKE3 hex.
///   There is no stake amount anywhere in this type.
/// - **PoWork = the hash of the work done** (`proof.work_proof.work_hash`).
///   There is no difficulty target and no leading-zero requirement.
/// - **PoSpace = WHERE.** Capacity is a descriptive attribute, never a gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosToken {
    /// Token identifier
    pub id: Vec<u8>,

    /// The canonical four-proof set (WHO / WHAT / WHERE / WHEN).
    pub proof: StateProof,

    /// Matrix position (x, y, z) this token is bound to in the Block-MATRIX
    /// topology — the transport-level expression of WHERE.
    pub matrix_position: (u32, u32, u32),

    /// Sequence number for wire ordering (transport-level WHEN).
    pub sequence: u64,

    /// Previous token hash for chain continuity (transport-level WHEN).
    pub prev_hash: Vec<u8>,

    /// FALCON-1024 public key of the authorized identity (PoStake = WHO).
    pub issuer_pubkey: Vec<u8>,

    /// Token signature (from issuer)
    pub signature: Vec<u8>,

    /// Token expiry time — how long the authorization is valid.
    pub expires_at: SystemTime,
}

impl PosToken {
    /// BLAKE3 hex of the issuer public key — the identity that
    /// `proof.stake_proof.stake_holder_id` must be bound to.
    pub fn issuer_identity(&self) -> String {
        blake3::hash(&self.issuer_pubkey).to_hex().to_string()
    }

    /// True iff the authorization identity (WHO) is bound to the issuer key.
    pub fn identity_is_bound(&self) -> bool {
        !self.issuer_pubkey.is_empty()
            && self.proof.stake_proof.stake_holder_id == self.issuer_identity()
    }

    /// Build a token whose PoStake authorization is bound to `issuer_pubkey`.
    ///
    /// Overwrites `proof.stake_proof.stake_holder_id` with the BLAKE3 hex of the
    /// issuer key so that WHO is always the identity that signs the token. The
    /// `signature` field is left empty for the caller to fill in.
    pub fn for_identity(
        id: Vec<u8>,
        issuer_pubkey: Vec<u8>,
        mut proof: StateProof,
        matrix_position: (u32, u32, u32),
        sequence: u64,
        prev_hash: Vec<u8>,
        valid_for: Duration,
    ) -> Self {
        proof.stake_proof.stake_holder_id = blake3::hash(&issuer_pubkey).to_hex().to_string();

        Self {
            id,
            proof,
            matrix_position,
            sequence,
            prev_hash,
            issuer_pubkey,
            signature: Vec::new(),
            expires_at: SystemTime::now() + valid_for,
        }
    }
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
        self.metrics
            .total_validations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Check cache first
        if let Some(cached) = self.validation_cache.get(&token.id) {
            if cached.expires_at > SystemTime::now() {
                self.metrics
                    .cache_hits
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                debug!("Token validation cache hit for {:?}", token.id);

                return Ok(ValidationResult {
                    is_valid: cached.result,
                    errors: if cached.result {
                        vec![]
                    } else {
                        vec!["Cached validation failure".to_string()]
                    },
                    validation_time: start_time.elapsed(),
                });
            }
        }

        self.metrics
            .cache_misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut errors = Vec::new();

        // 1. Check token expiry
        if token.expires_at <= SystemTime::now() {
            errors.push("Token has expired".to_string());
        }

        // 2. Validate the canonical four-proof set (WHERE / WHO / WHAT / WHEN).
        //    Structural validity only — no magnitude anywhere: no minimum
        //    stake, no difficulty target, no capacity floor.
        if !token.proof.space_proof.is_structurally_valid() {
            errors.push("Invalid Proof of Space".to_string());
        }
        if !token.proof.stake_proof.is_structurally_valid() {
            errors.push("Invalid Proof of Stake".to_string());
        }
        if !token.proof.work_proof.is_structurally_valid() {
            errors.push("Invalid Proof of Work".to_string());
        }
        if !token.proof.time_proof.is_structurally_valid() {
            errors.push("Invalid Proof of Time".to_string());
        }

        // 3. PoStake is an AUTHORIZATION binding: the identity claimed by the
        //    proof must be the BLAKE3 of the FALCON key that signed the token.
        if !token.identity_is_bound() {
            errors.push("PoStake identity not bound to issuer key".to_string());
        }

        // 4. Transport bindings: WHERE must name a matrix cell, and WHEN must
        //    carry chain continuity past genesis.
        if !self.validate_transport_bindings(token) {
            errors.push("Invalid transport binding (matrix position / continuity)".to_string());
        }

        // 5. Verify FALCON-1024 signature
        // Use TrustChain client if available, otherwise verify directly with
        // pqcrypto_falcon using the public key embedded in the token.
        let token_data = self.serialize_token_for_signing(token);
        let signer_pubkey = token.issuer_pubkey.as_slice();

        let sig_result = if let Some(ref client) = self.trustchain_client {
            client.verify_signature(signer_pubkey, &token_data, &token.signature)
        } else {
            verify_falcon_signature(signer_pubkey, &token_data, &token.signature)
        };

        match sig_result {
            Ok(true) => {
                debug!("Token signature verified successfully");
            }
            Ok(false) => {
                errors.push("Invalid token signature".to_string());
            }
            Err(e) => {
                errors.push(format!("Signature verification failed: {e}"));
            }
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
            self.metrics
                .failed_validations
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        let validation_time = start_time.elapsed();
        let validation_us = validation_time.as_micros() as u64;

        // Update average validation time (simple moving average)
        let current_avg = self
            .metrics
            .avg_validation_time_us
            .load(std::sync::atomic::Ordering::Relaxed);
        let new_avg = (current_avg * 9 + validation_us) / 10; // Weighted average
        self.metrics
            .avg_validation_time_us
            .store(new_avg, std::sync::atomic::Ordering::Relaxed);

        Ok(ValidationResult {
            is_valid,
            errors,
            validation_time,
        })
    }

    /// Validate the transport-level bindings carried alongside the canonical
    /// proof set.
    ///
    /// These are STOQ wire concerns, not proof magnitudes: WHERE must name a
    /// bound matrix cell, and WHEN must carry chain continuity past genesis.
    /// There is no capacity, difficulty, or stake threshold anywhere here.
    fn validate_transport_bindings(&self, token: &PosToken) -> bool {
        // WHERE: matrix position must be bound (non-zero location).
        if token.matrix_position == (0, 0, 0) {
            return false;
        }

        // WHEN: past genesis, chain continuity must be present.
        if token.sequence > 0 && token.prev_hash.is_empty() {
            return false;
        }

        true
    }

    /// Serialize token for signature verification using canonical length-prefixed format.
    ///
    /// Each field is encoded as a u32 LE length prefix followed by the field bytes.
    /// This prevents ambiguity from variable-length field concatenation.
    pub fn serialize_token_for_signing(&self, token: &PosToken) -> Vec<u8> {
        let mut buf = Vec::new();

        // Helper closure: write u32 LE length prefix then field bytes
        let mut write_field = |field: &[u8]| {
            buf.extend_from_slice(&(field.len() as u32).to_le_bytes());
            buf.extend_from_slice(field);
        };

        write_field(&token.id);
        write_field(token.proof.space_proof.file_hash.as_bytes());
        write_field(token.proof.stake_proof.stake_holder_id.as_bytes());
        write_field(&token.proof.work_proof.work_hash);
        write_field(&token.issuer_pubkey);
        write_field(&token.prev_hash);

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
            total_validations: self
                .metrics
                .total_validations
                .load(std::sync::atomic::Ordering::Relaxed),
            cache_hits: self
                .metrics
                .cache_hits
                .load(std::sync::atomic::Ordering::Relaxed),
            cache_misses: self
                .metrics
                .cache_misses
                .load(std::sync::atomic::Ordering::Relaxed),
            failed_validations: self
                .metrics
                .failed_validations
                .load(std::sync::atomic::Ordering::Relaxed),
            avg_validation_time_us: self
                .metrics
                .avg_validation_time_us
                .load(std::sync::atomic::Ordering::Relaxed),
            cache_size: self.validation_cache.len(),
        }
    }
}

/// Verify a FALCON-1024 signature directly using pqcrypto_falcon.
///
/// This performs local verification without requiring a TrustChain client connection.
/// Uses the same SHA-256 message hashing convention as `FalconTrustChainClient`.
fn verify_falcon_signature(pubkey: &[u8], data: &[u8], signature: &[u8]) -> Result<bool> {
    use pqcrypto_falcon::falcon1024;
    use pqcrypto_traits::sign::{DetachedSignature, PublicKey};
    use sha2::{Digest, Sha256};

    // Validate public key size (must be exactly FALCON-1024)
    if pubkey.len() != falcon1024::public_key_bytes() {
        return Ok(false);
    }

    // Reconstruct public key
    let public_key = falcon1024::PublicKey::from_bytes(pubkey)
        .map_err(|e| anyhow::anyhow!("Invalid FALCON-1024 public key: {e}"))?;

    // Reconstruct detached signature
    let detached_sig = match falcon1024::DetachedSignature::from_bytes(signature) {
        Ok(sig) => sig,
        Err(_) => return Ok(false),
    };

    // Hash the data (same convention as FalconTrustChainClient and FalconEngine)
    let mut hasher = Sha256::new();
    hasher.update(data);
    let message_hash: [u8; 32] = hasher.finalize().into();

    // Verify FALCON-1024 signature against message hash
    match falcon1024::verify_detached_signature(&detached_sig, &message_hash, &public_key) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
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


/// Test-only helpers shared by the PoS test modules across this crate.
#[cfg(test)]
pub(crate) mod test_support {
    use super::{PosToken, PosTokenValidator};
    use hypermesh_lib::proof::{SpaceProof, StakeProof, StateProof, TimeProof, WorkProof};
    use std::time::Duration;

    /// Build a canonical four-proof set.
    ///
    /// CANONICAL MODEL: authorization (WHO) is an identity binding with NO
    /// amount; WHAT is the BLAKE3 hash of the work performed; WHERE is a
    /// location (capacity is descriptive only, never a gate); WHEN is a time.
    pub(crate) fn canonical_test_proof() -> StateProof {
        let mut space = SpaceProof::new(
            "test-node-001".to_string(),
            "hypermesh://test-node-001/store".to_string(),
            1024 * 1024 * 1024,
        );
        space.file_hash = "a1b2c3d4e5f6".to_string();

        StateProof::new(
            StakeProof::new("test-owner".to_string(), "unbound".to_string()),
            TimeProof::new(Duration::from_secs(1)),
            space,
            WorkProof::from_work(
                "test-owner".to_string(),
                "test-workload".to_string(),
                b"the work that was actually done",
            ),
        )
    }

    /// Build a token whose PoStake identity is bound to a fresh FALCON-1024 key
    /// and whose signature genuinely verifies. Verification is mandatory, so
    /// tests that expect admission must present a real signature.
    pub(crate) fn signed_test_token(
        id: Vec<u8>,
        matrix_position: (u32, u32, u32),
        sequence: u64,
        prev_hash: Vec<u8>,
    ) -> PosToken {
        use pqcrypto_falcon::falcon1024;
        use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
        use sha2::{Digest, Sha256};

        let (pk, sk) = falcon1024::keypair();
        let mut token = PosToken::for_identity(
            id,
            pk.as_bytes().to_vec(),
            canonical_test_proof(),
            matrix_position,
            sequence,
            prev_hash,
            Duration::from_secs(3600),
        );

        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let data = validator.serialize_token_for_signing(&token);
        // The verifier signs over SHA-256(data) — match that convention.
        let digest: [u8; 32] = Sha256::digest(&data).into();
        let sk = falcon1024::SecretKey::from_bytes(sk.as_bytes())
            .expect("test: reconstruct secret key");
        token.signature = falcon1024::detached_sign(&digest, &sk).as_bytes().to_vec();
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::proof::{SpaceProof, StakeProof, TimeProof, WorkProof};

    use super::test_support::{canonical_test_proof, signed_test_token};

    /// Create an unsigned test token with valid structural fields but a bogus signature.
    fn create_unsigned_test_token() -> PosToken {
        let mut token = PosToken::for_identity(
            vec![1, 2, 3, 4],
            vec![20, 21, 22, 23],
            canonical_test_proof(),
            (1, 2, 3),
            1,
            vec![17, 18, 19, 20],
            Duration::from_secs(300),
        );
        token.signature = vec![21, 22, 23, 24];
        token
    }

    /// Create a properly FALCON-1024-signed test token.
    fn create_signed_test_token() -> PosToken {
        signed_test_token(vec![1, 2, 3, 4], (1, 2, 3), 1, vec![17, 18, 19, 20])
    }

    #[test]
    fn test_postake_is_authorization_not_amount() {
        // CANONICAL MODEL: PoStake carries NO magnitude. The whole of WHO is
        // the identity binding between the proof and the signing FALCON key.
        let token = create_signed_test_token();
        assert!(token.identity_is_bound());
        assert_eq!(
            token.proof.stake_proof.stake_holder_id,
            token.issuer_identity()
        );
    }

    #[test]
    fn test_zero_capacity_token_is_admitted() {
        // Capacity is a descriptive attribute, never an admission gate.
        use pqcrypto_falcon::falcon1024;
        use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
        use sha2::{Digest, Sha256};

        let validator = PosTokenValidator::new(Duration::from_secs(300));

        let (pk, sk) = falcon1024::keypair();
        let mut proof = canonical_test_proof();
        proof.space_proof.total_storage = 0;
        proof.space_proof.total_size = 0;

        let mut token = PosToken::for_identity(
            vec![9, 9, 9, 9],
            pk.as_bytes().to_vec(),
            proof,
            (1, 2, 3),
            1,
            vec![17, 18, 19, 20],
            Duration::from_secs(3600),
        );
        let data = validator.serialize_token_for_signing(&token);
        let digest: [u8; 32] = Sha256::digest(&data).into();
        let sk = falcon1024::SecretKey::from_bytes(sk.as_bytes())
            .expect("test: reconstruct secret key");
        token.signature = falcon1024::detached_sign(&digest, &sk).as_bytes().to_vec();

        let result = validator.validate_token(&token).expect("test: validation");
        assert!(
            result.is_valid,
            "zero capacity must not gate admission; errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_token_with_valid_falcon_signature() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let token = create_signed_test_token();

        let result = validator.validate_token(&token).expect("test: validation");
        assert!(result.is_valid, "Errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_token_with_bogus_signature_rejected() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let token = create_unsigned_test_token();

        let result = validator.validate_token(&token).expect("test: validation");
        assert!(!result.is_valid);
        assert!(
            result.errors.iter().any(|e| e.contains("signature")),
            "Expected signature error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_expired_token() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let mut token = create_signed_test_token();
        token.expires_at = SystemTime::now() - Duration::from_secs(60);

        let result = validator.validate_token(&token).expect("test: validation");
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("expired")));
    }

    #[test]
    fn test_cache_functionality() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let token = create_signed_test_token();

        // First validation should miss cache
        let result1 = validator.validate_token(&token).expect("test: validation");
        assert!(result1.is_valid, "Errors: {:?}", result1.errors);

        let stats = validator.get_metrics();
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hits, 0);

        // Second validation should hit cache
        let result2 = validator.validate_token(&token).expect("test: validation");
        assert!(result2.is_valid);

        let stats = validator.get_metrics();
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hits, 1);
    }

    #[test]
    fn test_validation_metrics() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let token = create_signed_test_token();

        // Perform multiple validations
        for _ in 0..5 {
            let _ = validator.validate_token(&token);
        }

        let stats = validator.get_metrics();
        assert_eq!(stats.total_validations, 5);
        let _ = stats.avg_validation_time_us;
    }

    #[test]
    fn test_tampered_token_rejected() {
        let validator = PosTokenValidator::new(Duration::from_secs(300));
        let mut token = create_signed_test_token();
        // Tamper with the token data after signing
        token.id = vec![99, 99, 99, 99];

        let result = validator.validate_token(&token).expect("test: validation");
        assert!(!result.is_valid);
        assert!(
            result.errors.iter().any(|e| e.contains("signature")),
            "Expected signature error after tampering, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_verify_falcon_signature_wrong_key_size() {
        // Wrong key size returns Ok(false), not an error
        let result =
            verify_falcon_signature(&[1, 2, 3], b"data", &[4, 5, 6]).expect("test: verify");
        assert!(!result);
    }
}
