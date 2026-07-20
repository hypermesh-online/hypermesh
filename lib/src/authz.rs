// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Authorization model — owner / grant / capacity.
//!
//! CANONICAL MODEL (asset-pos-model-canonical):
//! - OWNER holds the **distribution right** over an asset.
//! - A GRANTEE holds **access** — a grant of `Read`/`Use`/`Transfer` is the
//!   right to hold, PoS-validate, and serve (i.e. the grantee becomes a
//!   MIRROR). Access is authorization, never a magnitude.
//! - PoStake is AUTHORIZATION: a FALCON identity binding, never a stake
//!   amount / coin quantity. There is NO quota and NO coin logic here.
//! - CAPACITY is a descriptive asset attribute (an adapter's
//!   `CapacityProfile`), never a proof and never a gate.
//!
//! A [`Grant`] is signed with a [`GrantSig`] — the same recipe as the
//! `WireSignedProof` envelope used for state proofs: a FALCON-1024 detached
//! signature over `BLAKE3(proof_bytes || nonce)`, carrying the signer's
//! public key so the grant is cryptographically bound to the grantor.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Owner
// ---------------------------------------------------------------------------

/// An owner of an asset — holds the distribution right.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Owner {
    /// Identity of the owner (BLAKE3 hex of the FALCON-1024 public key).
    pub identity_id: String,
}

impl Owner {
    /// Create a new owner from an identity id.
    pub fn new(identity_id: impl Into<String>) -> Self {
        Self { identity_id: identity_id.into() }
    }
}

// ---------------------------------------------------------------------------
// GrantScope
// ---------------------------------------------------------------------------

/// What a grant authorizes. NO `Quota` — load balancing is emergent, never a
/// quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrantScope {
    /// Right to read the asset (hold + PoS-validate + serve as a mirror).
    Read,
    /// Right to use the asset (invoke / execute).
    Use,
    /// Right to transfer the asset (re-grant / hand off distribution).
    Transfer,
}

// ---------------------------------------------------------------------------
// GrantSig — WireSignedProof recipe applied to a grant
// ---------------------------------------------------------------------------

/// FALCON-1024 signature envelope over the canonical bytes of a [`Grant`].
///
/// Same recipe as the state-proof `WireSignedProof`: the signature is a
/// detached FALCON-1024 signature over `BLAKE3(proof_bytes || nonce)`, and the
/// signer's public key is carried so the grant is bound to the grantor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantSig {
    /// Canonical serialized grant bytes that were signed.
    pub proof_bytes: Vec<u8>,
    /// FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`.
    pub signature: Vec<u8>,
    /// Signer's full FALCON-1024 public key.
    pub signer_pubkey: Vec<u8>,
    /// Random nonce to prevent replay.
    pub nonce: [u8; 32],
}

impl GrantSig {
    /// Returns true iff this signature was produced by `pubkey` (raw FALCON
    /// bytes). Used to enforce that the grant signer is the grantor.
    pub fn signer_matches(&self, pubkey: &[u8]) -> bool {
        self.signer_pubkey == pubkey
    }
}

// ---------------------------------------------------------------------------
// Grant
// ---------------------------------------------------------------------------

/// A grant of access from an owner (`grantor`) to a `grantee`.
///
/// A grant of `Read`/`Use` makes the grantee a MIRROR — the right to hold,
/// PoS-validate, and serve the asset. It is authorization, never a magnitude.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// Identity of the granting owner.
    pub grantor: String,
    /// Identity being granted access.
    pub grantee: String,
    /// What is authorized.
    pub scope: GrantScope,
    /// Optional expiry — `None` means the grant does not expire.
    pub not_after: Option<SystemTime>,
    /// FALCON-1024 signature binding this grant to the grantor.
    pub signature: GrantSig,
}

impl Grant {
    /// Canonical bytes of the grant's authorizing fields (everything except
    /// the signature). This is what `signature.proof_bytes` must equal for a
    /// grant to be well-formed, and what gets signed.
    pub fn canonical_bytes(
        grantor: &str,
        grantee: &str,
        scope: GrantScope,
        not_after: Option<SystemTime>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(grantor.as_bytes());
        out.push(0);
        out.extend_from_slice(grantee.as_bytes());
        out.push(0);
        out.push(match scope {
            GrantScope::Read => 0,
            GrantScope::Use => 1,
            GrantScope::Transfer => 2,
        });
        let secs = not_after
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.extend_from_slice(&secs.to_le_bytes());
        out
    }

    /// Canonical bytes for *this* grant (from its own fields).
    pub fn my_canonical_bytes(&self) -> Vec<u8> {
        Self::canonical_bytes(&self.grantor, &self.grantee, self.scope, self.not_after)
    }

    /// The BLAKE3 digest that the FALCON signature covers, given the grant's
    /// canonical bytes and the signature nonce.
    fn signing_digest(proof_bytes: &[u8], nonce: &[u8; 32]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(proof_bytes);
        hasher.update(nonce);
        *hasher.finalize().as_bytes()
    }

    /// Digest for this grant's signature.
    pub fn signing_digest_bytes(&self) -> [u8; 32] {
        Self::signing_digest(&self.signature.proof_bytes, &self.signature.nonce)
    }

    /// True iff the signature's `proof_bytes` matches this grant's own
    /// canonical bytes (structural binding — does NOT verify the FALCON
    /// signature; crypto lives in TrustChain).
    pub fn proof_bytes_match(&self) -> bool {
        self.signature.proof_bytes == self.my_canonical_bytes()
    }

    /// True iff this grant is expired as of `now`.
    pub fn is_expired(&self, now: SystemTime) -> bool {
        matches!(self.not_after, Some(t) if now > t)
    }
}

// ---------------------------------------------------------------------------
// AuthorizationSet + AuthDecision
// ---------------------------------------------------------------------------

/// The complete authorization state for an asset: its owners and grants.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationSet {
    /// Owners holding the distribution right.
    pub owners: Vec<Owner>,
    /// Grants of access to grantees.
    pub grants: Vec<Grant>,
}

/// Result of an authorization check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthDecision {
    /// Access is authorized.
    Allowed,
    /// Access is denied, with a human-readable reason.
    Denied(String),
}

impl AuthDecision {
    /// True iff allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, AuthDecision::Allowed)
    }
}

impl AuthorizationSet {
    /// Create an authorization set with a single owner and no grants.
    pub fn with_owner(identity_id: impl Into<String>) -> Self {
        Self {
            owners: vec![Owner::new(identity_id)],
            grants: Vec::new(),
        }
    }

    /// Is `identity_id` an owner (distribution right)?
    pub fn is_owner(&self, identity_id: &str) -> bool {
        self.owners.iter().any(|o| o.identity_id == identity_id)
    }

    /// Structural authorization decision for `identity_id` requesting `scope`
    /// as of `now`.
    ///
    /// Owners are always allowed. Otherwise a non-expired grant to
    /// `identity_id` with a `scope` that covers the request authorizes it.
    /// `Transfer` covers `Use` and `Read`; `Use` covers `Read`.
    ///
    /// This is the structural decision only — it assumes grant signatures were
    /// already verified at ingest (crypto lives in TrustChain).
    pub fn decide(&self, identity_id: &str, scope: GrantScope, now: SystemTime) -> AuthDecision {
        if self.is_owner(identity_id) {
            return AuthDecision::Allowed;
        }
        let covered = self.grants.iter().any(|g| {
            g.grantee == identity_id
                && !g.is_expired(now)
                && scope_covers(g.scope, scope)
        });
        if covered {
            AuthDecision::Allowed
        } else {
            AuthDecision::Denied(format!(
                "{identity_id} is not an owner and holds no grant covering {scope:?}"
            ))
        }
    }
}

/// True iff a `held` grant scope authorizes a `requested` scope.
/// Ordering: `Transfer` ⊇ `Use` ⊇ `Read`.
fn scope_covers(held: GrantScope, requested: GrantScope) -> bool {
    fn rank(s: GrantScope) -> u8 {
        match s {
            GrantScope::Read => 0,
            GrantScope::Use => 1,
            GrantScope::Transfer => 2,
        }
    }
    rank(held) >= rank(requested)
}

// ---------------------------------------------------------------------------
// CapacityProfile — descriptive, never a proof
// ---------------------------------------------------------------------------

/// A single named capacity dimension of an asset (descriptive only).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityDimension {
    /// Name of the dimension (e.g. "cpu_cores", "storage_bytes").
    pub name: String,
    /// Total descriptive units in this dimension.
    pub total_units: u64,
}

/// Descriptive capacity of an asset. This is an attribute, NOT a proof and
/// NEVER a gate — capacity is never validated against a minimum.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityProfile {
    /// Capacity dimensions.
    pub dimensions: Vec<CapacityDimension>,
}

impl CapacityProfile {
    /// Total units advertised for `name`, if present.
    pub fn units(&self, name: &str) -> Option<u64> {
        self.dimensions.iter().find(|d| d.name == name).map(|d| d.total_units)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use pqcrypto_falcon::falcon1024;
    use pqcrypto_traits::sign::{DetachedSignature, PublicKey};

    /// Sign a grant's canonical bytes with a FALCON-1024 secret key, producing
    /// a well-formed `GrantSig`. Mirrors the `WireSignedProof` recipe.
    fn sign_grant(
        grantor: &str,
        grantee: &str,
        scope: GrantScope,
        not_after: Option<SystemTime>,
        pk: &falcon1024::PublicKey,
        sk: &falcon1024::SecretKey,
    ) -> Grant {
        let proof_bytes = Grant::canonical_bytes(grantor, grantee, scope, not_after);
        let mut nonce = [0u8; 32];
        nonce[0] = 7;
        let digest = Grant::signing_digest(&proof_bytes, &nonce);
        let sig = falcon1024::detached_sign(&digest, sk);
        Grant {
            grantor: grantor.to_string(),
            grantee: grantee.to_string(),
            scope,
            not_after,
            signature: GrantSig {
                proof_bytes,
                signature: sig.as_bytes().to_vec(),
                signer_pubkey: pk.as_bytes().to_vec(),
                nonce,
            },
        }
    }

    /// Verify a grant's FALCON signature (the TrustChain-side check, exercised
    /// here in-test to prove the recipe round-trips).
    fn verify_grant(grant: &Grant) -> bool {
        if !grant.proof_bytes_match() {
            return false;
        }
        let pk = match falcon1024::PublicKey::from_bytes(&grant.signature.signer_pubkey) {
            Ok(pk) => pk,
            Err(_) => return false,
        };
        let sig = match falcon1024::DetachedSignature::from_bytes(&grant.signature.signature) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let digest = grant.signing_digest_bytes();
        falcon1024::verify_detached_signature(&sig, &digest, &pk).is_ok()
    }

    #[test]
    fn grant_sign_verify_roundtrip() {
        let (pk, sk) = falcon1024::keypair();
        let grant = sign_grant("owner-1", "grantee-1", GrantScope::Read, None, &pk, &sk);
        assert!(verify_grant(&grant), "well-formed grant must verify");
        assert!(grant.proof_bytes_match(), "proof_bytes must equal canonical bytes");
    }

    #[test]
    fn grant_wrong_key_rejected() {
        // Signer key != grantor's claimed key: sign with sk1 but present pk2.
        let (_pk1, sk1) = falcon1024::keypair();
        let (pk2, _sk2) = falcon1024::keypair();
        let grant = sign_grant("owner-1", "grantee-1", GrantScope::Use, None, &pk2, &sk1);
        assert!(
            !verify_grant(&grant),
            "grant signed by a different key than it presents must be rejected"
        );
    }

    #[test]
    fn grant_tampered_scope_rejected() {
        let (pk, sk) = falcon1024::keypair();
        let mut grant = sign_grant("owner-1", "grantee-1", GrantScope::Read, None, &pk, &sk);
        // Escalate the scope after signing — proof_bytes no longer match.
        grant.scope = GrantScope::Transfer;
        assert!(!grant.proof_bytes_match(), "tampered scope breaks binding");
        assert!(!verify_grant(&grant), "tampered grant must be rejected");
    }

    #[test]
    fn owner_allowed_non_grantee_denied() {
        let now = SystemTime::now();
        let set = AuthorizationSet::with_owner("owner-1");
        assert!(set.decide("owner-1", GrantScope::Transfer, now).is_allowed());
        assert!(
            matches!(set.decide("stranger", GrantScope::Read, now), AuthDecision::Denied(_)),
            "test: stranger must be denied"
        );
    }

    #[test]
    fn grantee_allowed_within_scope() {
        let now = SystemTime::now();
        let (pk, sk) = falcon1024::keypair();
        let grant = sign_grant("owner-1", "reader-1", GrantScope::Use, None, &pk, &sk);
        let set = AuthorizationSet {
            owners: vec![Owner::new("owner-1")],
            grants: vec![grant],
        };
        // Use grant covers Read and Use, not Transfer.
        assert!(set.decide("reader-1", GrantScope::Read, now).is_allowed());
        assert!(set.decide("reader-1", GrantScope::Use, now).is_allowed());
        assert!(!set.decide("reader-1", GrantScope::Transfer, now).is_allowed());
    }

    #[test]
    fn expired_grant_denied() {
        let past = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);
        let now = SystemTime::now();
        let (pk, sk) = falcon1024::keypair();
        let grant = sign_grant("owner-1", "reader-1", GrantScope::Read, Some(past), &pk, &sk);
        let set = AuthorizationSet {
            owners: vec![Owner::new("owner-1")],
            grants: vec![grant],
        };
        assert!(!set.decide("reader-1", GrantScope::Read, now).is_allowed());
    }

    #[test]
    fn capacity_profile_is_descriptive() {
        let profile = CapacityProfile {
            dimensions: vec![
                CapacityDimension { name: "cpu_cores".into(), total_units: 8 },
                CapacityDimension { name: "storage_bytes".into(), total_units: 1_000_000 },
            ],
        };
        assert_eq!(profile.units("cpu_cores"), Some(8));
        assert_eq!(profile.units("missing"), None);
    }
}
