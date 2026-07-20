// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset authorization for BlockMatrix adapters.
//!
//! CANONICAL MODEL (asset-pos-model-canonical):
//! - OWNER holds the **distribution right**.
//! - A GRANTEE holds **access** — a grant of `Read`/`Use`/`Transfer` is the
//!   right to hold, PoS-validate, and serve (the grantee becomes a MIRROR).
//! - PoStake is AUTHORIZATION (FALCON identity binding), never a magnitude.
//!   There is NO quota / stake amount / coin logic anywhere in this layer.
//! - CAPACITY is a descriptive adapter attribute ([`CapacityProfile`]), never a
//!   proof and never a gate.
//!
//! The authorization *types* live in `hypermesh_lib::authz` (single source of
//! truth, re-exported here). This module adds the BlockMatrix adapter-side
//! decision logic:
//! - [`default_authorize`] — the generic `AssetAdapter::authorize` default.
//! - [`verify_grant`] — FALCON-1024 verification of a grant, using the same
//!   recipe as the H3 signed-proof envelope, plus the grantor-binding check
//!   (`BLAKE3(signer_pubkey) == grantor`) that mirrors `signer_binds_to_author`.

use std::time::SystemTime;

pub use hypermesh_lib::authz::{
    AuthDecision, AuthorizationSet, CapacityDimension, CapacityProfile, Grant, GrantScope,
    GrantSig, Owner,
};

use super::asset_id::AssetRegistration;

/// FALCON-verify a [`Grant`] and confirm it is bound to its grantor.
///
/// Returns `true` iff:
/// 1. the grant's `signature.proof_bytes` equal the grant's own canonical
///    bytes (no bait-and-switch on the authorizing fields),
/// 2. the FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`
///    verifies against the embedded `signer_pubkey`, and
/// 3. the signer binds to the grantor: `hex(BLAKE3(signer_pubkey)) == grantor`
///    (collapsed-identity model — the same binding `signer_binds_to_author`
///    enforces on block entries). Without this, any key could mint a grant
///    asserting someone else's distribution right.
pub fn verify_grant(grant: &Grant) -> bool {
    // (1) structural binding — signature covers this grant's authorizing fields.
    if !grant.proof_bytes_match() {
        return false;
    }

    // (2) FALCON-1024 over BLAKE3(proof_bytes || nonce).
    let mut hasher = blake3::Hasher::new();
    hasher.update(&grant.signature.proof_bytes);
    hasher.update(&grant.signature.nonce);
    let digest = hasher.finalize();

    let verified = <crate::identity::FalconIdentity as hypermesh_lib::NodeSigner>::verify_signature(
        &grant.signature.signer_pubkey,
        digest.as_bytes(),
        &grant.signature.signature,
    )
    .unwrap_or(false);
    if !verified {
        return false;
    }

    // (3) signer binds to grantor (distribution right cannot be forged).
    let derived = blake3::hash(&grant.signature.signer_pubkey)
        .to_hex()
        .to_string();
    grant.grantor == derived
}

/// Generic authorization decision for an adapter: is `actor` authorized to
/// perform `action` on `asset`?
///
/// Decision order (canonical model):
/// 1. `actor` is an OWNER → `Allowed` (owners hold the distribution right).
/// 2. Otherwise, a valid, non-expired grant to `actor` covering `action`,
///    whose FALCON signature verifies and whose signer binds to the grantor,
///    → `Allowed` (the grantee is an authorized MIRROR).
/// 3. Otherwise → `Denied`.
///
/// This is the default for every adapter; adapters may override for
/// asset-type-specific policy but none currently need to.
pub fn default_authorize(
    asset: &AssetRegistration,
    actor: &str,
    action: GrantScope,
) -> AuthDecision {
    let now = SystemTime::now();
    let auth = &asset.authorization;

    if auth.is_owner(actor) {
        return AuthDecision::Allowed;
    }

    let covered = auth.grants.iter().any(|g| {
        g.grantee == actor
            && !g.is_expired(now)
            && scope_covers(g.scope, action)
            && verify_grant(g)
    });

    if covered {
        AuthDecision::Allowed
    } else {
        AuthDecision::Denied(format!(
            "{actor} is not an owner and holds no valid grant covering {action:?}"
        ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::asset_id::AssetRegistration;
    use crate::assets::core::AssetType;
    use pqcrypto_falcon::falcon1024;
    use pqcrypto_traits::sign::{DetachedSignature, PublicKey};

    /// Build a FALCON-signed grant whose grantor binds to the signing key
    /// (`grantor = hex(BLAKE3(pubkey))`), the way a real owner would mint one.
    fn signed_grant(
        grantee: &str,
        scope: GrantScope,
        not_after: Option<SystemTime>,
    ) -> (String, Grant) {
        let (pk, sk) = falcon1024::keypair();
        let grantor = blake3::hash(pk.as_bytes()).to_hex().to_string();
        let proof_bytes = Grant::canonical_bytes(&grantor, grantee, scope, not_after);
        let mut nonce = [0u8; 32];
        nonce[0] = 9;
        let mut hasher = blake3::Hasher::new();
        hasher.update(&proof_bytes);
        hasher.update(&nonce);
        let digest = hasher.finalize();
        let sig = falcon1024::detached_sign(digest.as_bytes(), &sk);
        let grant = Grant {
            grantor: grantor.clone(),
            grantee: grantee.to_string(),
            scope,
            not_after,
            signature: GrantSig {
                proof_bytes,
                signature: sig.as_bytes().to_vec(),
                signer_pubkey: pk.as_bytes().to_vec(),
                nonce,
            },
        };
        (grantor, grant)
    }

    #[test]
    fn owner_is_allowed() {
        let asset = AssetRegistration::new(AssetType::Storage).with_owner("owner-1");
        assert!(default_authorize(&asset, "owner-1", GrantScope::Transfer).is_allowed());
    }

    #[test]
    fn non_grantee_is_denied() {
        let asset = AssetRegistration::new(AssetType::Storage).with_owner("owner-1");
        assert!(!default_authorize(&asset, "stranger", GrantScope::Read).is_allowed());
    }

    #[test]
    fn valid_grantee_within_scope_allowed() {
        let (_grantor, grant) = signed_grant("reader-1", GrantScope::Use, None);
        assert!(verify_grant(&grant), "well-formed grant must verify");
        let mut asset = AssetRegistration::new(AssetType::Storage).with_owner("owner-1");
        asset.authorization.grants.push(grant);
        // Use covers Read and Use, not Transfer.
        assert!(default_authorize(&asset, "reader-1", GrantScope::Read).is_allowed());
        assert!(default_authorize(&asset, "reader-1", GrantScope::Use).is_allowed());
        assert!(!default_authorize(&asset, "reader-1", GrantScope::Transfer).is_allowed());
    }

    #[test]
    fn grant_signer_not_bound_to_grantor_rejected() {
        // Sign a real grant, then rewrite the grantor to a stranger: signer no
        // longer binds to the grantor, so verify_grant must reject it.
        let (_grantor, mut grant) = signed_grant("reader-1", GrantScope::Read, None);
        grant.grantor = "not-the-signer".to_string();
        assert!(
            !verify_grant(&grant),
            "grant whose signer does not bind to grantor must be rejected"
        );
    }

    #[test]
    fn forged_grant_in_set_does_not_authorize() {
        // A grant present in the set but whose FALCON signature is garbage must
        // not authorize — default_authorize verifies signatures.
        let (_grantor, mut grant) = signed_grant("reader-1", GrantScope::Read, None);
        grant.signature.signature = vec![0u8; grant.signature.signature.len()];
        let mut asset = AssetRegistration::new(AssetType::Storage).with_owner("owner-1");
        asset.authorization.grants.push(grant);
        assert!(!default_authorize(&asset, "reader-1", GrantScope::Read).is_allowed());
    }

    #[test]
    fn authorization_excluded_from_identity_and_content_hash() {
        // Two registrations identical except for authorization must be the SAME
        // asset (equal, same hash) — authorization is never part of identity.
        let base = AssetRegistration::new_from_hash(&[7u8; 32]);
        let owned = base.clone().with_owner("owner-x");
        assert_eq!(base, owned, "authorization must not affect equality");

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        base.hash(&mut h1);
        let mut h2 = DefaultHasher::new();
        owned.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish(), "authorization must not affect hash");

        // content_hash field itself is byte-stable across authorization change.
        assert_eq!(
            base.content_hash, owned.content_hash,
            "content_hash must be byte-stable across authorization change"
        );
    }
}
