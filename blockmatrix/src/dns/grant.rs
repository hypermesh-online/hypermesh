// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Foundation DNS grant (Phase H.1).
//!
//! A [`FoundationGrant`] is a FALCON-1024-signed authorization issued
//! by the foundation that allows a specific identity to register a
//! reserved domain (see [`crate::dns::reserved`]). It carries the
//! recipient's FALCON public key so that `register_domain` can verify
//! the registering node actually owns the keypair the grant names.
//!
//! ## Lifecycle
//!
//! 1. Foundation operator runs `dns.foundation_grant` IPC (admin-only,
//!    requires `state.foundation_signing_key` to be configured).
//! 2. Daemon constructs the grant struct, signs the canonical payload
//!    with FALCON-1024, returns serialized grant + signature.
//! 3. Operator delivers the grant out-of-band to the recipient.
//! 4. Recipient calls `dns.register` (or `domain.register`) with the
//!    grant attached. `DnsRegistrar::register_domain` verifies:
//!     - reserved-domain match → grant required
//!     - signature valid against foundation pubkey
//!     - `recipient_pubkey` matches the signer of the registration
//!     - `valid_until` not in the past
//! 5. Daemon also publishes the grant as a Catalog asset using the
//!    `foundation.dns_grant/v1` typedef so other nodes can audit.
//!
//! ## Wire format of `signing_payload`
//!
//! `[domain || 0x00 || recipient_pubkey || 0x00 || valid_until_secs_le ||
//!  dues_paid_until_secs_le || issued_at_secs_le]`
//!
//! All `SystemTime` fields are encoded as 8-byte little-endian Unix
//! seconds. Domain and pubkey are NUL-terminated to make the byte-string
//! parseable in case future field reordering is needed; the canonical
//! verification path uses byte-exact match on `signing_payload()`.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Foundation-signed grant authorizing a recipient to register a
/// reserved domain.
///
/// Signed with FALCON-1024 — the foundation's signing key and the
/// recipient's pubkey are independent keypairs. The recipient's
/// pubkey is bound into the signed payload so a grant cannot be
/// transferred to a different identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundationGrant {
    /// The reserved domain name this grant authorizes.
    pub domain: String,
    /// FALCON-1024 public key of the recipient (raw bytes).
    pub recipient_pubkey: Vec<u8>,
    /// Grant expiry — registrations after this are rejected.
    pub valid_until: SystemTime,
    /// Last paid dues date — used by Phase F.2 CRL revocation when
    /// dues lapse. Present for forward compatibility; not enforced by
    /// the H.1 verification path.
    pub dues_paid_until: SystemTime,
    /// When the foundation signed this grant.
    pub issued_at: SystemTime,
    /// FALCON-1024 detached signature over `signing_payload()`.
    pub foundation_signature: Vec<u8>,
}

impl FoundationGrant {
    /// Construct a grant struct *without* a signature.
    ///
    /// Callers fill in `foundation_signature` separately, typically via
    /// `NodeSigner::sign(&grant.signing_payload())`.
    pub fn new_unsigned(
        domain: String,
        recipient_pubkey: Vec<u8>,
        valid_until: SystemTime,
        dues_paid_until: SystemTime,
    ) -> Self {
        Self {
            domain,
            recipient_pubkey,
            valid_until,
            dues_paid_until,
            issued_at: SystemTime::now(),
            foundation_signature: Vec::new(),
        }
    }

    /// Canonical byte string used for signing and verification.
    ///
    /// See module docs for wire layout. The same bytes must be produced
    /// for signing and for verification; any field reordering must bump
    /// the typedef version.
    pub fn signing_payload(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            self.domain.len() + 1 + self.recipient_pubkey.len() + 1 + 24,
        );
        buf.extend_from_slice(self.domain.as_bytes());
        buf.push(0u8);
        buf.extend_from_slice(&self.recipient_pubkey);
        buf.push(0u8);
        buf.extend_from_slice(&systime_to_secs_le(self.valid_until));
        buf.extend_from_slice(&systime_to_secs_le(self.dues_paid_until));
        buf.extend_from_slice(&systime_to_secs_le(self.issued_at));
        buf
    }

    /// Verify the grant's FALCON-1024 signature against the foundation
    /// pubkey.  Does NOT check `valid_until` against current time —
    /// callers do that separately so they can produce a more specific
    /// error (`ExpiredGrant` vs `InvalidGrant`).
    pub fn verify(&self, foundation_pubkey: &[u8]) -> bool {
        if self.foundation_signature.is_empty() {
            return false;
        }

        let payload = self.signing_payload();
        match <trustchain::FalconIdentity as hypermesh_lib::NodeSigner>::verify_signature(
            foundation_pubkey,
            &payload,
            &self.foundation_signature,
        ) {
            Ok(true) => true,
            Ok(false) => false,
            Err(_) => false,
        }
    }

    /// Whether the grant has expired (`valid_until` in the past).
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.valid_until
    }

    /// Whether the grant's `recipient_pubkey` matches the supplied
    /// FALCON public key (used to enforce that the registering identity
    /// is the grant recipient).
    pub fn recipient_matches(&self, candidate_pubkey: &[u8]) -> bool {
        // Constant-time comparison is overkill for public keys, but use
        // a length check first to avoid spurious mismatches on wire-format
        // edge cases.
        self.recipient_pubkey.len() == candidate_pubkey.len()
            && self.recipient_pubkey == candidate_pubkey
    }
}

fn systime_to_secs_le(t: SystemTime) -> [u8; 8] {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::NodeSigner;
    use std::time::Duration;
    use trustchain::FalconIdentity;

    fn fixed_grant(domain: &str, recipient: Vec<u8>) -> FoundationGrant {
        FoundationGrant::new_unsigned(
            domain.to_string(),
            recipient,
            SystemTime::now() + Duration::from_secs(365 * 24 * 60 * 60),
            SystemTime::now() + Duration::from_secs(365 * 24 * 60 * 60),
        )
    }

    #[test]
    fn signing_payload_is_deterministic() {
        let g = fixed_grant("nike", vec![1, 2, 3, 4]);
        let p1 = g.signing_payload();
        let p2 = g.signing_payload();
        assert_eq!(p1, p2);
    }

    #[test]
    fn signing_payload_changes_with_domain() {
        let g1 = fixed_grant("nike", vec![1, 2, 3, 4]);
        let mut g2 = g1.clone();
        g2.domain = "apple".to_string();
        assert_ne!(g1.signing_payload(), g2.signing_payload());
    }

    #[test]
    fn signing_payload_changes_with_recipient() {
        let g1 = fixed_grant("nike", vec![1, 2, 3, 4]);
        let g2 = fixed_grant("nike", vec![5, 6, 7, 8]);
        assert_ne!(g1.signing_payload(), g2.signing_payload());
    }

    #[test]
    fn unsigned_grant_does_not_verify() {
        let foundation = FalconIdentity::generate();
        let g = fixed_grant("nike", vec![1, 2, 3, 4]);
        assert!(!g.verify(&foundation.public_key));
    }

    #[test]
    fn signed_grant_verifies_against_correct_key() {
        let foundation = FalconIdentity::generate();
        let recipient = FalconIdentity::generate();

        let mut g = fixed_grant("nike", recipient.public_key.clone());
        let sig = foundation.sign(&g.signing_payload()).expect("test: sign");
        g.foundation_signature = sig;

        assert!(g.verify(&foundation.public_key));
    }

    #[test]
    fn signed_grant_does_not_verify_against_wrong_key() {
        let foundation = FalconIdentity::generate();
        let other = FalconIdentity::generate();
        let recipient = FalconIdentity::generate();

        let mut g = fixed_grant("nike", recipient.public_key.clone());
        let sig = foundation.sign(&g.signing_payload()).expect("test: sign");
        g.foundation_signature = sig;

        assert!(!g.verify(&other.public_key));
    }

    #[test]
    fn tampered_signature_fails_verification() {
        let foundation = FalconIdentity::generate();
        let recipient = FalconIdentity::generate();

        let mut g = fixed_grant("nike", recipient.public_key.clone());
        let mut sig = foundation.sign(&g.signing_payload()).expect("test: sign");
        // Flip a byte in the middle of the signature.
        if !sig.is_empty() {
            let mid = sig.len() / 2;
            sig[mid] ^= 0xff;
        }
        g.foundation_signature = sig;

        assert!(!g.verify(&foundation.public_key));
    }

    #[test]
    fn tampered_payload_fails_verification() {
        let foundation = FalconIdentity::generate();
        let recipient = FalconIdentity::generate();

        let mut g = fixed_grant("nike", recipient.public_key.clone());
        let sig = foundation.sign(&g.signing_payload()).expect("test: sign");
        g.foundation_signature = sig;
        // Tamper with the domain after signing.
        g.domain = "apple".to_string();

        assert!(!g.verify(&foundation.public_key));
    }

    #[test]
    fn recipient_matches_exact_only() {
        let g = fixed_grant("nike", vec![1, 2, 3, 4]);
        assert!(g.recipient_matches(&[1, 2, 3, 4]));
        assert!(!g.recipient_matches(&[1, 2, 3, 5]));
        assert!(!g.recipient_matches(&[1, 2, 3]));
        assert!(!g.recipient_matches(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn expired_grant_detected() {
        let mut g = fixed_grant("nike", vec![]);
        g.valid_until = SystemTime::now() - Duration::from_secs(60);
        assert!(g.is_expired());
    }

    #[test]
    fn future_grant_not_expired() {
        let g = fixed_grant("nike", vec![]);
        assert!(!g.is_expired());
    }
}
