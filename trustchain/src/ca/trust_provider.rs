// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! `TrustSignalProvider` — engauge-driven federation gating.
//!
//! Phase F.1 wires per-peer activity, capacity, and traffic-classification
//! signals from engauge into TrustChain's `FederationManager`.  The trait
//! lives in trustchain so the federation manager doesn't need to depend
//! on engauge directly; the concrete adapter lives in blockmatrix
//! (`intelligence::engauge_trust_adapter`) and converts engauge data into
//! the small descriptor below.
//!
//! TrustChain only needs to know the resulting [`PeerTrustBand`] — a
//! coarse Full/Conditional band that maps directly onto
//! [`crate::ca::federation::FederationTrustLevel`].  The fine-grained
//! engauge signal types stay on the engauge side.

use async_trait::async_trait;

/// A peer fingerprint used to key trust lookups.
///
/// In practice this is the SHA-256 fingerprint of the peer's CA public
/// key, identical to the `[u8; 32]` keys used by `FederationManager`'s
/// held-key-shares map.
pub type PeerCertFingerprint = [u8; 32];

/// Coarse trust band returned by the signal provider.
///
/// `Full` and `Conditional` mirror engauge's `TrustBand`.  Trustchain
/// keeps its own enum so this module compiles without an engauge
/// dependency.  `Untrusted` is reserved for the byzantine override path
/// inside `FederationManager::add_peer` — providers must not return it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerTrustBand {
    /// Promote the peer to the requested trust level (subject to PoS).
    Full,
    /// Cap the peer at `Conditional` regardless of requested level.
    Conditional,
}

/// Provider of engauge-derived trust signals for a single peer.
///
/// The federation manager queries this asynchronously each time a peer
/// is added, so the provider may consult shared state (e.g. an
/// `EngaugeBridge`) under a lock.  Returning `None` means "no signals
/// available" — the federation manager falls back to its existing
/// PoS-only gating in that case.
#[async_trait]
pub trait TrustSignalProvider: Send + Sync {
    /// Look up the trust band for a peer by CA fingerprint.
    async fn trust_band_for(&self, peer: &PeerCertFingerprint) -> Option<PeerTrustBand>;
}
