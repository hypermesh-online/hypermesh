// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Capability tokens (Phase K.1).
//!
//! Scope-bounded session tokens issued by a daemon and bound to a device
//! pubkey. Each token carries:
//!
//! - `session_id` — UUIDv4 identifier
//! - `device_pubkey` — FALCON-1024 pubkey of the device the daemon is
//!   issuing the token to (so a stolen token cannot be replayed by a
//!   different device)
//! - `capabilities` — `Vec<Capability>` (`ViewOnly | Wallet | AssetWrite |
//!   Admin`); `Admin` grants all
//! - `issued_at`, `valid_until` — timestamps
//! - `issued_by` — daemon's FALCON-1024 pubkey (so the recipient can pin
//!   it to a specific node)
//! - `signature` — FALCON-1024 detached signature over [`signing_payload`]
//!
//! ## Wire format of `signing_payload`
//!
//! ```text
//! session_id (16 bytes)
//!   || 0x00
//!   || device_pubkey
//!   || 0x00
//!   || capabilities_sorted (one byte per cap, ascending)
//!   || 0x00
//!   || issued_at_secs_le (8 bytes)
//!   || valid_until_secs_le (8 bytes)
//!   || issued_by
//! ```
//!
//! Capabilities are sorted by their u8 discriminant before serialization
//! so two tokens with the same effective scope produce the same payload.
//!
//! ## Verification
//!
//! [`CapabilityToken::verify`] takes the daemon's FALCON pubkey and
//! returns `true` only when the FALCON-1024 detached signature is valid
//! over `signing_payload`. Callers separately check expiry via
//! [`CapabilityToken::is_expired`] so they can produce a more specific
//! error.
//!
//! ## Revocation
//!
//! Sessions can be revoked via `auth.revoke_session` IPC which records a
//! [`SessionAction::Revoked`] entry on-chain *and* inserts the
//! `session_id` into the in-memory [`RevocationRegistry`]. The IPC
//! middleware is expected to consult both the signature (via `verify`)
//! and the revocation registry before granting access.
//!
//! ## Alpha-default inert
//!
//! Issuing a token requires a configured [`CapabilityTokenIssuer`]
//! (which holds the daemon's FALCON identity). The IPC handler guards
//! on this — if no issuer is configured, `auth.create_session` rejects
//! with "auth not configured".

#![deny(unsafe_code)]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

use trustchain::FalconIdentity;

use hypermesh_lib::NodeSigner;

/// Error type for capability-token operations.
#[derive(Debug, Error)]
pub enum CapabilityTokenError {
    /// Token signature did not verify against the supplied pubkey.
    #[error("invalid token signature")]
    InvalidSignature,
    /// Token expiry is in the past.
    #[error("token expired")]
    Expired,
    /// Token signature payload could not be constructed (system clock).
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    /// FALCON sign failed (bad key material).
    #[error("FALCON sign failed: {0}")]
    SignFailed(String),
    /// Capability not granted (caller required a higher scope).
    #[error("capability {required:?} not in token scope {granted:?}")]
    CapabilityDenied {
        /// What the protected method demands.
        required: Capability,
        /// What the token actually carries.
        granted: Vec<Capability>,
    },
    /// Token's session_id is in the revocation registry.
    #[error("session {0} has been revoked")]
    Revoked(Uuid),
}

/// A capability is a coarse-grained authorization scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Capability {
    /// Read-only access — list assets, query chain, read dashboards.
    ViewOnly,
    /// Caesar wallet operations — view balance, send/receive payments.
    Wallet,
    /// Write asset entries — store, share, register DNS, etc.
    AssetWrite,
    /// All capabilities. Admin can do anything ViewOnly/Wallet/AssetWrite
    /// can plus reserved-for-admin actions (e.g. `system.apply_update`).
    Admin,
}

impl Capability {
    /// One-byte discriminant used in `signing_payload`. Stable across
    /// versions; new variants append.
    pub fn as_u8(self) -> u8 {
        match self {
            Self::ViewOnly => 0x01,
            Self::Wallet => 0x02,
            Self::AssetWrite => 0x03,
            Self::Admin => 0xFF,
        }
    }
}

/// Session-audit action — what happened to the session. Used by the
/// `SessionAudit` chain entry payload so a user can prove their own
/// audit trail of "which device, with which capabilities, did what,
/// when" by inspecting their own chain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SessionAction {
    /// Session token was issued.
    Created,
    /// Session was explicitly revoked (admin or self).
    Revoked,
    /// A capability of the session was exercised (audited per-method).
    CapabilityUsed {
        /// IPC method name that was invoked under the session.
        method: String,
    },
}

/// FALCON-signed scope-bounded session token.
///
/// See module docs for wire format and verification rules.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Session UUID.
    pub session_id: Uuid,
    /// FALCON-1024 pubkey of the device the token is bound to.
    pub device_pubkey: Vec<u8>,
    /// Granted capabilities (deduplicated, but order preserved as issued).
    pub capabilities: Vec<Capability>,
    /// Token issuance time.
    pub issued_at: SystemTime,
    /// Token expiry time.
    pub valid_until: SystemTime,
    /// FALCON-1024 pubkey of the daemon that signed this token.
    pub issued_by: Vec<u8>,
    /// FALCON-1024 detached signature over [`signing_payload`].
    pub signature: Vec<u8>,
}

impl CapabilityToken {
    /// Construct an unsigned token. Callers fill in `signature`
    /// separately (typically via [`CapabilityTokenIssuer`]).
    pub fn new_unsigned(
        device_pubkey: Vec<u8>,
        capabilities: Vec<Capability>,
        issued_at: SystemTime,
        valid_until: SystemTime,
        issued_by: Vec<u8>,
    ) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            device_pubkey,
            capabilities,
            issued_at,
            valid_until,
            issued_by,
            signature: Vec::new(),
        }
    }

    /// Canonical byte string used for signing and verification.
    ///
    /// See module docs for layout. Capabilities are sorted ascending by
    /// their u8 discriminant before serialization.
    pub fn signing_payload(&self) -> Vec<u8> {
        let mut sorted_caps: Vec<u8> = self.capabilities.iter().map(|c| c.as_u8()).collect();
        sorted_caps.sort_unstable();
        sorted_caps.dedup();

        let issued_at_secs = systime_to_secs(self.issued_at);
        let valid_until_secs = systime_to_secs(self.valid_until);

        let mut buf = Vec::with_capacity(
            16 + 1
                + self.device_pubkey.len()
                + 1
                + sorted_caps.len()
                + 1
                + 8
                + 8
                + self.issued_by.len(),
        );
        buf.extend_from_slice(self.session_id.as_bytes());
        buf.push(0u8);
        buf.extend_from_slice(&self.device_pubkey);
        buf.push(0u8);
        buf.extend_from_slice(&sorted_caps);
        buf.push(0u8);
        buf.extend_from_slice(&issued_at_secs.to_le_bytes());
        buf.extend_from_slice(&valid_until_secs.to_le_bytes());
        buf.extend_from_slice(&self.issued_by);
        buf
    }

    /// Verify the FALCON-1024 detached signature against the daemon's
    /// pubkey. Does NOT check expiry — call [`is_expired`] separately.
    pub fn verify(&self, daemon_pubkey: &[u8]) -> bool {
        if self.signature.is_empty() {
            return false;
        }
        let payload = self.signing_payload();
        match <FalconIdentity as NodeSigner>::verify_signature(
            daemon_pubkey,
            &payload,
            &self.signature,
        ) {
            Ok(ok) => ok,
            Err(_) => false,
        }
    }

    /// True if `valid_until` is in the past.
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(SystemTime::now())
    }

    /// Variant of [`is_expired`] testable with an injected reference time.
    pub fn is_expired_at(&self, now: SystemTime) -> bool {
        match now.duration_since(self.valid_until) {
            Ok(_) => true,    // now >= valid_until → expired
            Err(_) => false,  // valid_until > now → still good
        }
    }

    /// True when the token grants the `required` capability.
    ///
    /// `Admin` is a superset that grants all other capabilities.
    /// Otherwise an exact match is required.
    pub fn allows(&self, required: &Capability) -> bool {
        if self.capabilities.contains(&Capability::Admin) {
            return true;
        }
        self.capabilities.contains(required)
    }

    /// Combined verify + expiry + revocation check, returning a
    /// specific error variant for each failure mode.
    pub fn validate(
        &self,
        daemon_pubkey: &[u8],
        revocations: &HashSet<Uuid>,
    ) -> Result<(), CapabilityTokenError> {
        if !self.verify(daemon_pubkey) {
            return Err(CapabilityTokenError::InvalidSignature);
        }
        if self.is_expired() {
            return Err(CapabilityTokenError::Expired);
        }
        if revocations.contains(&self.session_id) {
            return Err(CapabilityTokenError::Revoked(self.session_id));
        }
        Ok(())
    }
}

fn systime_to_secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Issues capability tokens signed with the daemon's FALCON identity.
///
/// Constructed once at daemon startup and stored in `DaemonState`. When
/// the issuer is `None` the `auth.create_session` IPC rejects with
/// "auth not configured".
pub struct CapabilityTokenIssuer {
    daemon_identity: Arc<FalconIdentity>,
}

impl CapabilityTokenIssuer {
    /// Construct an issuer wrapping the daemon's FALCON identity.
    pub fn new(daemon_identity: Arc<FalconIdentity>) -> Self {
        Self { daemon_identity }
    }

    /// Return the daemon's FALCON pubkey — the verifier key for any
    /// token this issuer produces.
    pub fn daemon_pubkey(&self) -> &[u8] {
        &self.daemon_identity.public_key
    }

    /// Issue a new token bound to `device_pubkey` with the given
    /// capabilities and a `ttl` lifetime.
    pub fn issue(
        &self,
        device_pubkey: Vec<u8>,
        capabilities: Vec<Capability>,
        ttl: Duration,
    ) -> Result<CapabilityToken, CapabilityTokenError> {
        let issued_at = SystemTime::now();
        let valid_until = issued_at + ttl;

        // Deduplicate capabilities, preserving first-occurrence order.
        let mut seen: HashSet<Capability> = HashSet::new();
        let mut deduped: Vec<Capability> = Vec::with_capacity(capabilities.len());
        for cap in capabilities {
            if seen.insert(cap) {
                deduped.push(cap);
            }
        }

        let mut token = CapabilityToken::new_unsigned(
            device_pubkey,
            deduped,
            issued_at,
            valid_until,
            self.daemon_identity.public_key.clone(),
        );
        let payload = token.signing_payload();
        let sig = self
            .daemon_identity
            .sign(&payload)
            .map_err(|e| CapabilityTokenError::SignFailed(e.to_string()))?;
        token.signature = sig;
        Ok(token)
    }
}

/// In-memory revocation registry.
///
/// Holds `session_id`s that have been explicitly revoked (via
/// `auth.revoke_session`). The IPC layer consults this on every
/// validation.  K.1.5 will rebuild the registry from the on-chain
/// `SessionAudit` log at daemon startup.
#[derive(Default)]
pub struct RevocationRegistry {
    revoked: Arc<RwLock<HashSet<Uuid>>>,
}

impl RevocationRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `session_id` as revoked. Idempotent.
    pub async fn revoke(&self, session_id: Uuid) {
        self.revoked.write().await.insert(session_id);
    }

    /// True when the session has been revoked.
    pub async fn is_revoked(&self, session_id: &Uuid) -> bool {
        self.revoked.read().await.contains(session_id)
    }

    /// Snapshot of the current revocation set (for use with
    /// [`CapabilityToken::validate`]).
    pub async fn snapshot(&self) -> HashSet<Uuid> {
        self.revoked.read().await.clone()
    }

    /// Number of revoked sessions.
    pub async fn len(&self) -> usize {
        self.revoked.read().await.len()
    }

    /// True when nothing has been revoked.
    pub async fn is_empty(&self) -> bool {
        self.revoked.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> Arc<FalconIdentity> {
        Arc::new(FalconIdentity::generate())
    }

    fn fake_device_pubkey() -> Vec<u8> {
        FalconIdentity::generate().public_key.clone()
    }

    #[test]
    fn admin_implies_all() {
        let now = SystemTime::now();
        let tok = CapabilityToken::new_unsigned(
            vec![1u8; 8],
            vec![Capability::Admin],
            now,
            now + Duration::from_secs(60),
            vec![2u8; 8],
        );
        assert!(tok.allows(&Capability::ViewOnly));
        assert!(tok.allows(&Capability::Wallet));
        assert!(tok.allows(&Capability::AssetWrite));
        assert!(tok.allows(&Capability::Admin));
    }

    #[test]
    fn view_only_denies_write() {
        let now = SystemTime::now();
        let tok = CapabilityToken::new_unsigned(
            vec![1u8; 8],
            vec![Capability::ViewOnly],
            now,
            now + Duration::from_secs(60),
            vec![2u8; 8],
        );
        assert!(tok.allows(&Capability::ViewOnly));
        assert!(!tok.allows(&Capability::AssetWrite));
        assert!(!tok.allows(&Capability::Admin));
    }

    #[test]
    fn issue_round_trip_verifies() {
        let identity = test_identity();
        let issuer = CapabilityTokenIssuer::new(identity.clone());
        let device_pubkey = fake_device_pubkey();
        let token = issuer
            .issue(
                device_pubkey,
                vec![Capability::Wallet, Capability::ViewOnly],
                Duration::from_secs(60),
            )
            .expect("test: issue");

        // Verify against issuer pubkey.
        assert!(token.verify(&identity.public_key));

        // Round-trip through serialization.
        let json = serde_json::to_vec(&token).expect("test: serialize");
        let back: CapabilityToken = serde_json::from_slice(&json).expect("test: deserialize");
        assert!(back.verify(&identity.public_key));
        assert_eq!(back.session_id, token.session_id);
    }

    #[test]
    fn signature_tampering_detected() {
        let identity = test_identity();
        let issuer = CapabilityTokenIssuer::new(identity.clone());
        let mut token = issuer
            .issue(
                fake_device_pubkey(),
                vec![Capability::ViewOnly],
                Duration::from_secs(60),
            )
            .expect("test: issue");
        // Flip a byte.
        token.signature[0] ^= 0xFF;
        assert!(!token.verify(&identity.public_key));
    }

    #[test]
    fn expiry_check() {
        let now = SystemTime::now();
        let past = now - Duration::from_secs(60);
        let tok = CapabilityToken::new_unsigned(
            vec![1u8; 8],
            vec![Capability::ViewOnly],
            past,
            past, // valid_until = past → expired
            vec![2u8; 8],
        );
        assert!(tok.is_expired());

        let future = now + Duration::from_secs(60);
        let tok2 = CapabilityToken::new_unsigned(
            vec![1u8; 8],
            vec![Capability::ViewOnly],
            now,
            future,
            vec![2u8; 8],
        );
        assert!(!tok2.is_expired());
    }

    #[tokio::test]
    async fn revocation_registry_round_trip() {
        let reg = RevocationRegistry::new();
        let sid = Uuid::new_v4();
        assert!(!reg.is_revoked(&sid).await);
        reg.revoke(sid).await;
        assert!(reg.is_revoked(&sid).await);
        // Idempotent.
        reg.revoke(sid).await;
        assert_eq!(reg.len().await, 1);
    }

    #[test]
    fn capability_dedup_in_issuer() {
        let identity = test_identity();
        let issuer = CapabilityTokenIssuer::new(identity.clone());
        let token = issuer
            .issue(
                fake_device_pubkey(),
                vec![
                    Capability::ViewOnly,
                    Capability::Wallet,
                    Capability::ViewOnly, // duplicate
                ],
                Duration::from_secs(60),
            )
            .expect("test: issue");
        assert_eq!(token.capabilities.len(), 2);
    }

    #[test]
    fn signing_payload_capabilities_canonical_order() {
        let now = SystemTime::now();
        let valid_until = now + Duration::from_secs(60);

        let t1 = CapabilityToken {
            session_id: Uuid::nil(),
            device_pubkey: vec![1u8; 4],
            capabilities: vec![Capability::Wallet, Capability::ViewOnly],
            issued_at: now,
            valid_until,
            issued_by: vec![2u8; 4],
            signature: Vec::new(),
        };
        let t2 = CapabilityToken {
            session_id: Uuid::nil(),
            device_pubkey: vec![1u8; 4],
            capabilities: vec![Capability::ViewOnly, Capability::Wallet],
            issued_at: now,
            valid_until,
            issued_by: vec![2u8; 4],
            signature: Vec::new(),
        };
        assert_eq!(t1.signing_payload(), t2.signing_payload());
    }
}
