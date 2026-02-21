// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Self-sovereign KYC attestation.
//!
//! Attestations live on the Device chain.  The network only ever sees a hash
//! of the attestation -- no PII leaves the node.
//!
//! [`ComplianceChecker`] validates attestation hashes against expiry windows
//! and maps [`MarketTier`] requirements to [`AttestationLevel`]s.

use chrono::{DateTime, Utc};
use hypermesh_lib::{ContentHash, MarketTier};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AttestationLevel
// ---------------------------------------------------------------------------

/// Graduated KYC assurance levels.  No PII is encoded here -- just the
/// level of verification the node claims to have undergone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AttestationLevel {
    /// Self-declared identity only.
    Basic,
    /// Third-party verification (document scan, liveness check).
    Enhanced,
    /// Institutional-grade due diligence.
    Institutional,
}

// ---------------------------------------------------------------------------
// KycAttestation
// ---------------------------------------------------------------------------

/// Hash-only KYC attestation stored on the Device chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KycAttestation {
    /// BLAKE3 hash of the full attestation document (stored off-chain).
    pub attestation_hash: ContentHash,
    /// When the attestation was issued.
    pub attested_at: DateTime<Utc>,
    /// When the attestation expires.
    pub expiry: DateTime<Utc>,
    /// Assurance level.
    pub level: AttestationLevel,
}

// ---------------------------------------------------------------------------
// ComplianceResult
// ---------------------------------------------------------------------------

/// Outcome of checking a single [`KycAttestation`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplianceResult {
    /// Whether the attestation passed all checks.
    pub valid: bool,
    /// The level of the attestation.
    pub level: AttestationLevel,
    /// Seconds remaining until the attestation expires (0 if already expired).
    pub expires_in_secs: u64,
}

// ---------------------------------------------------------------------------
// ComplianceChecker
// ---------------------------------------------------------------------------

/// Validates KYC attestations and maps market tiers to required levels.
#[derive(Debug, Clone)]
pub struct ComplianceChecker;

impl ComplianceChecker {
    /// Create a new checker.
    pub fn new() -> Self {
        Self
    }

    /// Check an attestation's validity at the current time.
    pub fn check_attestation(&self, attestation: &KycAttestation) -> ComplianceResult {
        let valid = self.is_valid(attestation);
        let now = Utc::now();
        let expires_in_secs = if attestation.expiry > now {
            (attestation.expiry - now).num_seconds().max(0) as u64
        } else {
            0
        };

        ComplianceResult {
            valid,
            level: attestation.level,
            expires_in_secs,
        }
    }

    /// Quick validity check: hash must be non-zero and not expired.
    pub fn is_valid(&self, attestation: &KycAttestation) -> bool {
        if attestation.attestation_hash == ContentHash::zeroed() {
            return false;
        }
        Utc::now() < attestation.expiry
    }

    /// What attestation level is required for a given market tier?
    pub fn required_level_for_tier(&self, tier: MarketTier) -> AttestationLevel {
        match tier {
            MarketTier::L0 => AttestationLevel::Basic,
            MarketTier::L1 => AttestationLevel::Enhanced,
            MarketTier::L2 | MarketTier::L3 => AttestationLevel::Institutional,
        }
    }
}

impl Default for ComplianceChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn valid_attestation() -> KycAttestation {
        KycAttestation {
            attestation_hash: ContentHash::from_bytes([1u8; 32]),
            attested_at: Utc::now() - Duration::hours(1),
            expiry: Utc::now() + Duration::days(30),
            level: AttestationLevel::Enhanced,
        }
    }

    fn expired_attestation() -> KycAttestation {
        KycAttestation {
            attestation_hash: ContentHash::from_bytes([2u8; 32]),
            attested_at: Utc::now() - Duration::days(60),
            expiry: Utc::now() - Duration::days(1),
            level: AttestationLevel::Basic,
        }
    }

    #[test]
    fn valid_attestation_passes() {
        let checker = ComplianceChecker::new();
        let att = valid_attestation();
        assert!(checker.is_valid(&att));
        let result = checker.check_attestation(&att);
        assert!(result.valid);
        assert!(result.expires_in_secs > 0);
        assert_eq!(result.level, AttestationLevel::Enhanced);
    }

    #[test]
    fn expired_attestation_fails() {
        let checker = ComplianceChecker::new();
        let att = expired_attestation();
        assert!(!checker.is_valid(&att));
        let result = checker.check_attestation(&att);
        assert!(!result.valid);
        assert_eq!(result.expires_in_secs, 0);
    }

    #[test]
    fn zeroed_hash_fails() {
        let checker = ComplianceChecker::new();
        let att = KycAttestation {
            attestation_hash: ContentHash::zeroed(),
            attested_at: Utc::now(),
            expiry: Utc::now() + Duration::days(30),
            level: AttestationLevel::Institutional,
        };
        assert!(!checker.is_valid(&att));
    }

    #[test]
    fn tier_to_level_mapping() {
        let checker = ComplianceChecker::new();
        assert_eq!(
            checker.required_level_for_tier(MarketTier::L0),
            AttestationLevel::Basic
        );
        assert_eq!(
            checker.required_level_for_tier(MarketTier::L1),
            AttestationLevel::Enhanced
        );
        assert_eq!(
            checker.required_level_for_tier(MarketTier::L2),
            AttestationLevel::Institutional
        );
        assert_eq!(
            checker.required_level_for_tier(MarketTier::L3),
            AttestationLevel::Institutional
        );
    }

    #[test]
    fn attestation_level_ordering() {
        assert!(AttestationLevel::Basic < AttestationLevel::Enhanced);
        assert!(AttestationLevel::Enhanced < AttestationLevel::Institutional);
    }

    #[test]
    fn attestation_serde_roundtrip() {
        let att = valid_attestation();
        let json = serde_json::to_string(&att).expect("test: serialize attestation");
        let back: KycAttestation =
            serde_json::from_str(&json).expect("test: deserialize attestation");
        assert_eq!(att.attestation_hash, back.attestation_hash);
        assert_eq!(att.level, back.level);
    }

    #[test]
    fn compliance_result_serde_roundtrip() {
        let result = ComplianceResult {
            valid: true,
            level: AttestationLevel::Enhanced,
            expires_in_secs: 86400,
        };
        let json = serde_json::to_string(&result).expect("test: serialize result");
        let back: ComplianceResult =
            serde_json::from_str(&json).expect("test: deserialize result");
        assert_eq!(result, back);
    }
}
