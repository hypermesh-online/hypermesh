// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Fast PoS Token Pre-Validation for Line-Rate Filtering
//!
//! Two-stage validation: fast structural pre-check for line-rate filtering,
//! then full crypto validation via [`PosTokenValidator`]. Privacy-tier-aware:
//! Anonymous skips, Private does subset checks, Public does full 4-proof.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use tracing::{debug, warn};

use crate::protocol::pos_validator::{PosToken, PosTokenValidator, ValidationResult};
use hypermesh_lib::{AccessScope, PrivacyMode};

/// Configuration for the fast pre-validation stage.
#[derive(Debug, Clone)]
pub struct FastValidationConfig {
    /// Maximum clock skew tolerance in seconds (default: 60).
    pub max_clock_skew_secs: u64,
    /// Maximum token age in seconds (default: 86400 = 24h).
    pub max_token_age_secs: u64,
    /// Maximum serialized token size in bytes (default: 65536).
    pub max_token_size_bytes: usize,
    /// TTL for cached validation results in seconds (default: 60).
    pub fast_cache_ttl_secs: u64,
    /// Maximum validations per second before rate limiting (default: 10000).
    pub rate_limit_per_sec: u32,
}

impl Default for FastValidationConfig {
    fn default() -> Self {
        Self {
            max_clock_skew_secs: 60,
            max_token_age_secs: 86400,
            max_token_size_bytes: 65536,
            fast_cache_ttl_secs: 60,
            rate_limit_per_sec: 10_000,
        }
    }
}

/// Result of the fast (structural) pre-validation stage.
#[derive(Debug)]
pub enum FastValidationResult {
    /// Token passed structural checks and needs full crypto validation.
    PassToFull,
    /// Token failed structural checks; reject immediately with reason.
    Rejected(String),
    /// Token was previously validated and is still within cache TTL.
    CachedValid,
}

/// Aggregate statistics from the fast validator.
#[derive(Debug, Clone)]
pub struct FastValidatorStats {
    /// Total fast validations attempted.
    pub total_validations: u64,
    /// Total rejections at the fast stage.
    pub total_rejections: u64,
    /// Total cache hits (skipped full validation).
    pub cache_hits: u64,
    /// Current cache size.
    pub cache_size: usize,
}

/// Two-stage PoS validator: fast structural pre-check then full crypto.
///
/// The fast stage runs cheap checks (timestamp, size, rate limit,
/// authorization binding) and maintains a result cache so repeated tokens skip the
/// expensive full validation. Privacy-tier routing selects which proofs to
/// verify: Anonymous skips all, Private checks a subset, Public runs full.
pub struct PosFastValidator {
    config: FastValidationConfig,
    /// Cache: blake3 hash of token -> (valid, insertion time).
    fast_cache: Arc<DashMap<[u8; 32], (bool, SystemTime)>>,
    validation_count: AtomicU64,
    rejection_count: AtomicU64,
    cache_hit_count: AtomicU64,
    last_rate_reset: RwLock<Instant>,
    rate_count: AtomicU64,
    full_validator: Arc<PosTokenValidator>,
}

impl PosFastValidator {
    /// Create a new fast validator wrapping the given full validator.
    pub fn new(config: FastValidationConfig, full_validator: Arc<PosTokenValidator>) -> Self {
        Self {
            config,
            fast_cache: Arc::new(DashMap::new()),
            validation_count: AtomicU64::new(0),
            rejection_count: AtomicU64::new(0),
            cache_hit_count: AtomicU64::new(0),
            last_rate_reset: RwLock::new(Instant::now()),
            rate_count: AtomicU64::new(0),
            full_validator,
        }
    }

    /// Run structural pre-checks only (no crypto).
    ///
    /// Order: cache -> rate limit -> size -> timestamp -> authorization.
    pub fn fast_validate(&self, token: &PosToken) -> FastValidationResult {
        self.validation_count.fetch_add(1, Ordering::Relaxed);

        // 1. Cache lookup
        let token_hash = Self::hash_token(token);
        if let Some(entry) = self.fast_cache.get(&token_hash) {
            let (valid, inserted_at) = *entry;
            let ttl = Duration::from_secs(self.config.fast_cache_ttl_secs);
            if let Ok(age) = SystemTime::now().duration_since(inserted_at) {
                if age < ttl && valid {
                    self.cache_hit_count.fetch_add(1, Ordering::Relaxed);
                    return FastValidationResult::CachedValid;
                }
            }
        }

        // 2. Rate limit
        if let Some(reason) = self.check_rate_limit() {
            self.rejection_count.fetch_add(1, Ordering::Relaxed);
            return FastValidationResult::Rejected(reason);
        }

        // 3. Token size
        if let Some(reason) = self.check_token_size(token) {
            self.rejection_count.fetch_add(1, Ordering::Relaxed);
            return FastValidationResult::Rejected(reason);
        }

        // 4. Timestamp checks
        if let Some(reason) = self.check_timestamp(&token.proof.time_proof.time_verification_timestamp) {
            self.rejection_count.fetch_add(1, Ordering::Relaxed);
            return FastValidationResult::Rejected(reason);
        }

        // 5. Authorization binding (PoStake = WHO, not a magnitude): the token
        //    must carry a bound owner/grantee identity. Admission is
        //    authorized-or-not; the FALCON signature over that identity is
        //    verified in the full validation stage.
        if token.proof.stake_proof.stake_holder_id.is_empty() || token.issuer_pubkey.is_empty() {
            self.rejection_count.fetch_add(1, Ordering::Relaxed);
            return FastValidationResult::Rejected(
                "PoStake missing authorized identity binding".to_string(),
            );
        }

        FastValidationResult::PassToFull
    }

    /// Two-stage validation: fast pre-check then privacy-tier-aware full check.
    pub fn validate(
        &self,
        token: &PosToken,
        privacy_mode: &PrivacyMode,
    ) -> Result<ValidationResult> {
        let start = Instant::now();

        match self.fast_validate(token) {
            FastValidationResult::CachedValid => {
                debug!("Fast validator: cache hit");
                Ok(ValidationResult {
                    is_valid: true,
                    errors: vec![],
                    validation_time: start.elapsed(),
                })
            }
            FastValidationResult::Rejected(reason) => {
                warn!("Fast validator rejected: {}", reason);
                Err(anyhow!("Fast validation rejected: {reason}"))
            }
            FastValidationResult::PassToFull => {
                let result = self.validate_for_tier(token, privacy_mode)?;
                let token_hash = Self::hash_token(token);
                self.cache_result(token_hash, result.is_valid);
                Ok(result)
            }
        }
    }

    /// Privacy-tier-aware full validation.
    ///
    /// - **Anonymous**: skip all validation, return valid immediately.
    /// - **Private**: check authorization binding and timestamp only.
    /// - **Public**: full 4-proof validation via the underlying validator.
    pub fn validate_for_tier(
        &self,
        token: &PosToken,
        privacy_mode: &PrivacyMode,
    ) -> Result<ValidationResult> {
        let start = Instant::now();

        // Anonymous: skip everything
        if privacy_mode.scope == AccessScope::Unbounded && !privacy_mode.tracked {
            debug!("Anonymous mode: skipping PoS validation");
            return Ok(ValidationResult {
                is_valid: true,
                errors: vec![],
                validation_time: start.elapsed(),
            });
        }

        // Private: subset checks (authorization binding + timestamp only)
        if privacy_mode.scope == AccessScope::Bounded && privacy_mode.tracked {
            let mut errors = Vec::new();

            if token.proof.stake_proof.stake_holder_id.is_empty() || token.issuer_pubkey.is_empty()
            {
                errors.push("PoStake missing authorized identity binding".to_string());
            }

            let now = SystemTime::now();
            let max_skew = Duration::from_secs(self.config.max_clock_skew_secs);
            let max_age = Duration::from_secs(self.config.max_token_age_secs);

            if token.proof.time_proof.time_verification_timestamp > now + max_skew {
                errors.push("Timestamp is in the future".to_string());
            }
            if let Ok(age) = now.duration_since(token.proof.time_proof.time_verification_timestamp) {
                if age > max_age {
                    errors.push("Timestamp is too old".to_string());
                }
            }

            debug!("Private mode: subset validation (authorization + timestamp)");
            return Ok(ValidationResult {
                is_valid: errors.is_empty(),
                errors,
                validation_time: start.elapsed(),
            });
        }

        // Public (or any other combo): full 4-proof validation
        debug!("Public mode: full PoS validation");
        self.full_validator.validate_token(token)
    }

    /// Insert a validation result into the fast cache.
    pub fn cache_result(&self, token_hash: [u8; 32], valid: bool) {
        self.fast_cache
            .insert(token_hash, (valid, SystemTime::now()));
    }

    /// Remove expired entries from the fast cache.
    pub fn cleanup_cache(&self) {
        let ttl = Duration::from_secs(self.config.fast_cache_ttl_secs);
        let now = SystemTime::now();
        self.fast_cache.retain(|_, (_, inserted_at)| {
            now.duration_since(*inserted_at)
                .map(|age| age < ttl)
                .unwrap_or(false)
        });
    }

    /// Return current validator statistics.
    pub fn stats(&self) -> FastValidatorStats {
        FastValidatorStats {
            total_validations: self.validation_count.load(Ordering::Relaxed),
            total_rejections: self.rejection_count.load(Ordering::Relaxed),
            cache_hits: self.cache_hit_count.load(Ordering::Relaxed),
            cache_size: self.fast_cache.len(),
        }
    }

    /// Get a reference to the underlying full validator.
    pub fn full_validator(&self) -> &Arc<PosTokenValidator> {
        &self.full_validator
    }

    // --- private helpers ---

    /// Compute a blake3 hash of the token for cache keying.
    fn hash_token(token: &PosToken) -> [u8; 32] {
        let serialized = bincode::serialize(token).unwrap_or_default();
        blake3::hash(&serialized).into()
    }

    /// Check the per-second rate limit. Returns `Some(reason)` on rejection.
    fn check_rate_limit(&self) -> Option<String> {
        let now = Instant::now();
        {
            let mut last_reset = self.last_rate_reset.write();
            if now.duration_since(*last_reset) >= Duration::from_secs(1) {
                *last_reset = now;
                self.rate_count.store(0, Ordering::Relaxed);
            }
        }
        let count = self.rate_count.fetch_add(1, Ordering::Relaxed);
        if count >= self.config.rate_limit_per_sec as u64 {
            return Some(format!(
                "Rate limit exceeded: {} per second",
                self.config.rate_limit_per_sec,
            ));
        }
        None
    }

    /// Check serialized token size. Returns `Some(reason)` on rejection.
    fn check_token_size(&self, token: &PosToken) -> Option<String> {
        let size = bincode::serialized_size(token).unwrap_or(0) as usize;
        if size > self.config.max_token_size_bytes {
            return Some(format!(
                "Token size {} exceeds maximum {}",
                size, self.config.max_token_size_bytes,
            ));
        }
        None
    }

    /// Check timestamp bounds. Returns `Some(reason)` on rejection.
    fn check_timestamp(&self, timestamp: &SystemTime) -> Option<String> {
        let now = SystemTime::now();
        let max_skew = Duration::from_secs(self.config.max_clock_skew_secs);
        let max_age = Duration::from_secs(self.config.max_token_age_secs);

        if *timestamp > now + max_skew {
            return Some(format!(
                "Timestamp is {:.0}s in the future (max skew: {}s)",
                timestamp
                    .duration_since(now)
                    .unwrap_or_default()
                    .as_secs_f64(),
                self.config.max_clock_skew_secs,
            ));
        }

        if let Ok(age) = now.duration_since(*timestamp) {
            if age > max_age {
                return Some(format!(
                    "Timestamp is {}s old (max age: {}s)",
                    age.as_secs(),
                    self.config.max_token_age_secs,
                ));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::protocol::pos_validator::test_support::{canonical_test_proof, signed_test_token};

    /// Helper: create a valid test token that passes all fast checks.
    fn create_valid_token() -> PosToken {
        PosToken::for_identity(
            vec![1, 2, 3, 4],
            vec![25, 26, 27, 28],
            canonical_test_proof(),
            (1, 2, 3),
            1,
            vec![17, 18, 19, 20],
            Duration::from_secs(3600),
        )
    }

    /// Create a properly FALCON-1024-signed token for tests that reach full validation.
    fn create_falcon_signed_token() -> PosToken {
        signed_test_token(vec![1, 2, 3, 4], (1, 2, 3), 1, vec![17, 18, 19, 20])
    }

    fn make_validator() -> PosFastValidator {
        let full = Arc::new(PosTokenValidator::new(Duration::from_secs(300)));
        PosFastValidator::new(FastValidationConfig::default(), full)
    }

    #[test]
    fn test_fast_check_authorized_pass() {
        let v = make_validator();
        let token = create_valid_token();
        let result = v.fast_validate(&token);
        assert!(
            matches!(result, FastValidationResult::PassToFull),
            "Token with a bound authorized identity should pass to full validation"
        );
    }

    #[test]
    fn test_fast_check_missing_authorization_fail() {
        let v = make_validator();
        let mut token = create_valid_token();
        // Remove the authorized identity binding (PoStake = WHO). Admission is
        // authorized-or-not — never a magnitude threshold.
        token.proof.stake_proof.stake_holder_id.clear();
        let result = v.fast_validate(&token);
        assert!(
            matches!(result, FastValidationResult::Rejected(ref r) if r.contains("identity")),
            "Token missing its authorized identity binding should be rejected"
        );
    }

    #[test]
    fn test_fast_check_timestamp_future() {
        let v = make_validator();
        let mut token = create_valid_token();
        // 5 minutes in the future, beyond 60s max skew
        token.proof.time_proof.time_verification_timestamp = SystemTime::now() + Duration::from_secs(300);
        let result = v.fast_validate(&token);
        assert!(
            matches!(result, FastValidationResult::Rejected(ref r) if r.contains("future")),
            "Future timestamp should be rejected"
        );
    }

    #[test]
    fn test_fast_check_timestamp_too_old() {
        let v = make_validator();
        let mut token = create_valid_token();
        // 2 days old, beyond 24h max age
        token.proof.time_proof.time_verification_timestamp = SystemTime::now() - Duration::from_secs(172_800);
        let result = v.fast_validate(&token);
        assert!(
            matches!(result, FastValidationResult::Rejected(ref r) if r.contains("old")),
            "Old timestamp should be rejected"
        );
    }

    #[test]
    fn test_fast_check_size_limit() {
        let config = FastValidationConfig {
            max_token_size_bytes: 50, // Unrealistically small
            ..Default::default()
        };
        let full = Arc::new(PosTokenValidator::new(Duration::from_secs(300)));
        let v = PosFastValidator::new(config, full);

        let token = create_valid_token();
        let result = v.fast_validate(&token);
        assert!(
            matches!(result, FastValidationResult::Rejected(ref r) if r.contains("size")),
            "Oversized token should be rejected"
        );
    }

    #[test]
    fn test_fast_check_missing_authorization() {
        let v = make_validator();
        let mut token = create_valid_token();
        token.proof.stake_proof.stake_holder_id = String::new();
        let result = v.fast_validate(&token);
        assert!(
            matches!(result, FastValidationResult::Rejected(ref r) if r.contains("authorized")),
            "Missing authorization binding should be rejected"
        );
    }

    #[test]
    fn test_rate_limiting() {
        let config = FastValidationConfig {
            rate_limit_per_sec: 5,
            ..Default::default()
        };
        let full = Arc::new(PosTokenValidator::new(Duration::from_secs(300)));
        let v = PosFastValidator::new(config, full);

        let token = create_valid_token();
        // First 5 should pass (or use different tokens to avoid cache)
        for i in 0..5 {
            let mut t = token.clone();
            t.id = vec![i as u8];
            let result = v.fast_validate(&t);
            assert!(
                !matches!(result, FastValidationResult::Rejected(ref r) if r.contains("Rate")),
                "Validation {i} should not be rate-limited"
            );
        }
        // 6th should be rate-limited
        let mut t6 = token.clone();
        t6.id = vec![99];
        let result = v.fast_validate(&t6);
        assert!(
            matches!(result, FastValidationResult::Rejected(ref r) if r.contains("Rate")),
            "Should be rate-limited after exceeding limit"
        );
    }

    #[test]
    fn test_cache_hit() {
        let v = make_validator();
        let token = create_valid_token();

        // First call: should pass to full
        let r1 = v.fast_validate(&token);
        assert!(matches!(r1, FastValidationResult::PassToFull));

        // Manually cache the result
        let hash = PosFastValidator::hash_token(&token);
        v.cache_result(hash, true);

        // Second call: should hit cache
        let r2 = v.fast_validate(&token);
        assert!(
            matches!(r2, FastValidationResult::CachedValid),
            "Second validation should be a cache hit"
        );
        assert_eq!(v.stats().cache_hits, 1);
    }

    #[test]
    fn test_cache_expiry() {
        let config = FastValidationConfig {
            fast_cache_ttl_secs: 0, // Immediate expiry
            ..Default::default()
        };
        let full = Arc::new(PosTokenValidator::new(Duration::from_secs(300)));
        let v = PosFastValidator::new(config, full);

        let token = create_valid_token();
        let hash = PosFastValidator::hash_token(&token);

        // Insert with TTL=0 so it expires immediately
        v.cache_result(hash, true);

        // Sleep briefly to ensure expiry
        std::thread::sleep(Duration::from_millis(10));

        // Should NOT be a cache hit because TTL is 0
        let result = v.fast_validate(&token);
        assert!(
            !matches!(result, FastValidationResult::CachedValid),
            "Expired cache entry should not produce a hit"
        );
    }

    #[test]
    fn test_tier_anonymous_skip() {
        let v = make_validator();
        let token = create_valid_token();

        let result = v
            .validate(&token, &PrivacyMode::ANONYMOUS)
            .expect("test: anonymous validation should succeed");

        assert!(result.is_valid, "Anonymous mode should skip validation");
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_tier_private_subset() {
        let v = make_validator();
        let token = create_valid_token();

        let result = v
            .validate(&token, &PrivacyMode::PRIVATE)
            .expect("test: private validation should succeed");

        // Valid token should pass private subset checks
        assert!(result.is_valid, "Valid token should pass private checks");

        // Now test with a missing authorization binding (should fail).
        let mut bad_token = create_valid_token();
        bad_token.proof.stake_proof.stake_holder_id = String::new();
        // A missing identity binding is caught at the fast stage before the tier
        // check, so exercise validate_for_tier directly for the subset check.
        let result2 = v
            .validate_for_tier(&bad_token, &PrivacyMode::PRIVATE)
            .expect("test: private subset should return result");
        assert!(
            !result2.is_valid,
            "Missing authorization should fail private checks"
        );
        assert!(result2.errors.iter().any(|e| e.contains("authorized")));
    }

    #[test]
    fn test_tier_public_full() {
        let v = make_validator();
        let token = create_falcon_signed_token();

        let result = v
            .validate(&token, &PrivacyMode::PUBLIC)
            .expect("test: public validation should succeed");

        // Valid token with real FALCON signature should pass full validation
        assert!(
            result.is_valid,
            "Valid token should pass full public validation, errors: {:?}",
            result.errors
        );

        // Test with expired token (should fail full validation)
        // Use a different ID so the full validator's cache doesn't mask the failure.
        let mut expired = create_falcon_signed_token();
        expired.id = vec![99, 98, 97, 96];
        expired.expires_at = SystemTime::now() - Duration::from_secs(60);
        let result2 = v
            .validate_for_tier(&expired, &PrivacyMode::PUBLIC)
            .expect("test: public expired should return result");
        assert!(
            !result2.is_valid,
            "Expired token should fail public validation"
        );
    }
}
