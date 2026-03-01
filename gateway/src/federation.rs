// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Federation bridge for cross-network STOQ message forwarding.
//!
//! Manages peer federations and controls which ones are allowed to send/receive
//! forwarded messages through this gateway. Trust levels mirror those in the
//! [`crate::scope_router`] module.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tracing::{debug, info, warn};

use crate::error::GatewayError;
use crate::scope_router::GatewayTrustLevel;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A federation peer known to this gateway.
#[derive(Debug, Clone)]
pub struct FederationPeer {
    pub federation_id: String,
    pub name: String,
    /// STOQ endpoint address (e.g. `[::1]:8444`).
    pub endpoint: String,
    pub trust_level: GatewayTrustLevel,
    pub joined_at: Instant,
    pub last_seen: Instant,
}

/// Snapshot of federation bridge statistics.
#[derive(Debug, Clone)]
pub struct FederationStatsSnapshot {
    pub messages_forwarded: u64,
    pub messages_rejected: u64,
    pub peers_joined: u64,
    pub peers_removed: u64,
    pub active_peers: usize,
}

// ---------------------------------------------------------------------------
// Internal stats
// ---------------------------------------------------------------------------

struct FederationStats {
    messages_forwarded: AtomicU64,
    messages_rejected: AtomicU64,
    peers_joined: AtomicU64,
    peers_removed: AtomicU64,
}

impl FederationStats {
    fn new() -> Self {
        Self {
            messages_forwarded: AtomicU64::new(0),
            messages_rejected: AtomicU64::new(0),
            peers_joined: AtomicU64::new(0),
            peers_removed: AtomicU64::new(0),
        }
    }
}

// ---------------------------------------------------------------------------
// FederationBridge
// ---------------------------------------------------------------------------

/// Manages cross-federation STOQ message forwarding.
///
/// Each peer is identified by a `federation_id`. The bridge enforces trust
/// levels (Untrusted peers cannot join) and a configurable maximum peer count.
pub struct FederationBridge {
    local_federation_id: String,
    peers: Arc<DashMap<String, FederationPeer>>,
    max_peers: usize,
    stats: Arc<FederationStats>,
}

impl FederationBridge {
    /// Create a bridge for the given local federation, capping peers at
    /// `max_peers`.
    pub fn new(local_federation_id: String, max_peers: usize) -> Self {
        Self {
            local_federation_id,
            peers: Arc::new(DashMap::new()),
            max_peers,
            stats: Arc::new(FederationStats::new()),
        }
    }

    /// Join a federation by registering a peer.
    ///
    /// # Errors
    ///
    /// * [`GatewayError::Config`] if `max_peers` would be exceeded.
    /// * [`GatewayError::AuthFailed`] if the peer has `Untrusted` trust level.
    pub fn join_federation(&self, peer: FederationPeer) -> Result<(), GatewayError> {
        if self.peers.len() >= self.max_peers {
            return Err(GatewayError::Config(format!(
                "federation peer limit ({}) reached",
                self.max_peers
            )));
        }
        if peer.trust_level == GatewayTrustLevel::Untrusted {
            return Err(GatewayError::AuthFailed {
                reason: "cannot join untrusted federation".into(),
            });
        }
        info!(
            "federation peer joined: {} ({})",
            peer.federation_id, peer.name
        );
        self.peers.insert(peer.federation_id.clone(), peer);
        self.stats.peers_joined.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Leave a federation. Returns `true` if the peer existed.
    pub fn leave_federation(&self, federation_id: &str) -> bool {
        let removed = self.peers.remove(federation_id).is_some();
        if removed {
            self.stats.peers_removed.fetch_add(1, Ordering::Relaxed);
            info!("federation peer left: {}", federation_id);
        }
        removed
    }

    /// Check whether forwarding to a peer is allowed.
    ///
    /// A peer must be registered and must not be `Untrusted`.
    pub fn can_forward_to(&self, federation_id: &str) -> bool {
        self.peers
            .get(federation_id)
            .map(|p| p.trust_level != GatewayTrustLevel::Untrusted)
            .unwrap_or(false)
    }

    /// Record a forwarded message to the given federation.
    ///
    /// Updates the peer's `last_seen` timestamp and increments the forwarded
    /// counter. Returns an error if forwarding is not allowed.
    pub fn record_forward(&self, federation_id: &str) -> Result<(), GatewayError> {
        if !self.can_forward_to(federation_id) {
            self.stats.messages_rejected.fetch_add(1, Ordering::Relaxed);
            warn!("forwarding denied to federation '{}'", federation_id);
            return Err(GatewayError::AuthFailed {
                reason: format!("forwarding to federation '{federation_id}' denied"),
            });
        }
        if let Some(mut peer) = self.peers.get_mut(federation_id) {
            peer.last_seen = Instant::now();
        }
        self.stats
            .messages_forwarded
            .fetch_add(1, Ordering::Relaxed);
        debug!("message forwarded to federation '{}'", federation_id);
        Ok(())
    }

    /// List all currently registered peers.
    pub fn list_peers(&self) -> Vec<FederationPeer> {
        self.peers.iter().map(|e| e.value().clone()).collect()
    }

    /// Update the trust level for an existing peer.
    ///
    /// Returns `true` if the peer was found and updated, `false` otherwise.
    pub fn update_trust_level(&self, federation_id: &str, level: GatewayTrustLevel) -> bool {
        if let Some(mut peer) = self.peers.get_mut(federation_id) {
            debug!(
                "trust level updated for '{}': {:?} -> {:?}",
                federation_id, peer.trust_level, level
            );
            peer.trust_level = level;
            true
        } else {
            false
        }
    }

    /// Return a snapshot of bridge statistics.
    pub fn federation_stats(&self) -> FederationStatsSnapshot {
        FederationStatsSnapshot {
            messages_forwarded: self.stats.messages_forwarded.load(Ordering::Relaxed),
            messages_rejected: self.stats.messages_rejected.load(Ordering::Relaxed),
            peers_joined: self.stats.peers_joined.load(Ordering::Relaxed),
            peers_removed: self.stats.peers_removed.load(Ordering::Relaxed),
            active_peers: self.peers.len(),
        }
    }

    /// The local federation ID this bridge belongs to.
    pub fn local_id(&self) -> &str {
        &self.local_federation_id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(id: &str, name: &str, trust: GatewayTrustLevel) -> FederationPeer {
        FederationPeer {
            federation_id: id.into(),
            name: name.into(),
            endpoint: "[::1]:8444".into(),
            trust_level: trust,
            joined_at: Instant::now(),
            last_seen: Instant::now(),
        }
    }

    #[test]
    fn join_and_list_peers() {
        let bridge = FederationBridge::new("local".into(), 10);
        bridge
            .join_federation(make_peer("alpha", "Alpha Net", GatewayTrustLevel::Full))
            .expect("test: join alpha");
        bridge
            .join_federation(make_peer(
                "beta",
                "Beta Net",
                GatewayTrustLevel::Conditional,
            ))
            .expect("test: join beta");

        let peers = bridge.list_peers();
        assert_eq!(peers.len(), 2);
    }

    #[test]
    fn join_untrusted_rejected() {
        let bridge = FederationBridge::new("local".into(), 10);
        let result =
            bridge.join_federation(make_peer("bad", "Bad Net", GatewayTrustLevel::Untrusted));
        assert!(result.is_err());
        assert_eq!(bridge.list_peers().len(), 0);
    }

    #[test]
    fn max_peers_enforced() {
        let bridge = FederationBridge::new("local".into(), 2);
        bridge
            .join_federation(make_peer("a", "A", GatewayTrustLevel::Full))
            .expect("test: join a");
        bridge
            .join_federation(make_peer("b", "B", GatewayTrustLevel::Full))
            .expect("test: join b");
        let result = bridge.join_federation(make_peer("c", "C", GatewayTrustLevel::Full));
        assert!(result.is_err());
    }

    #[test]
    fn leave_federation() {
        let bridge = FederationBridge::new("local".into(), 10);
        bridge
            .join_federation(make_peer("alpha", "Alpha", GatewayTrustLevel::Full))
            .expect("test: join");
        assert!(bridge.leave_federation("alpha"));
        assert!(!bridge.leave_federation("alpha")); // already removed
        assert_eq!(bridge.list_peers().len(), 0);
    }

    #[test]
    fn can_forward_to_checks_trust() {
        let bridge = FederationBridge::new("local".into(), 10);
        bridge
            .join_federation(make_peer("full", "Full", GatewayTrustLevel::Full))
            .expect("test: join full");
        bridge
            .join_federation(make_peer(
                "cond",
                "Conditional",
                GatewayTrustLevel::Conditional,
            ))
            .expect("test: join cond");

        assert!(bridge.can_forward_to("full"));
        assert!(bridge.can_forward_to("cond"));
        // Not registered
        assert!(!bridge.can_forward_to("unknown"));
    }

    #[test]
    fn can_forward_to_returns_false_after_demote() {
        let bridge = FederationBridge::new("local".into(), 10);
        bridge
            .join_federation(make_peer("peer", "Peer", GatewayTrustLevel::Full))
            .expect("test: join");
        assert!(bridge.can_forward_to("peer"));

        bridge.update_trust_level("peer", GatewayTrustLevel::Untrusted);
        assert!(!bridge.can_forward_to("peer"));
    }

    #[test]
    fn record_forward_success() {
        let bridge = FederationBridge::new("local".into(), 10);
        bridge
            .join_federation(make_peer("alpha", "Alpha", GatewayTrustLevel::Full))
            .expect("test: join");
        bridge.record_forward("alpha").expect("test: forward");
        bridge.record_forward("alpha").expect("test: forward 2");

        let stats = bridge.federation_stats();
        assert_eq!(stats.messages_forwarded, 2);
        assert_eq!(stats.messages_rejected, 0);
    }

    #[test]
    fn record_forward_denied_for_unknown() {
        let bridge = FederationBridge::new("local".into(), 10);
        let result = bridge.record_forward("nonexistent");
        assert!(result.is_err());
        assert_eq!(bridge.federation_stats().messages_rejected, 1);
    }

    #[test]
    fn update_trust_level() {
        let bridge = FederationBridge::new("local".into(), 10);
        bridge
            .join_federation(make_peer("alpha", "Alpha", GatewayTrustLevel::Full))
            .expect("test: join");
        assert!(bridge.update_trust_level("alpha", GatewayTrustLevel::Conditional));
        assert!(!bridge.update_trust_level("unknown", GatewayTrustLevel::Full));

        let peers = bridge.list_peers();
        let alpha = peers
            .iter()
            .find(|p| p.federation_id == "alpha")
            .expect("test: find alpha");
        assert_eq!(alpha.trust_level, GatewayTrustLevel::Conditional);
    }

    #[test]
    fn stats_snapshot() {
        let bridge = FederationBridge::new("local".into(), 10);
        let s = bridge.federation_stats();
        assert_eq!(s.messages_forwarded, 0);
        assert_eq!(s.messages_rejected, 0);
        assert_eq!(s.peers_joined, 0);
        assert_eq!(s.peers_removed, 0);
        assert_eq!(s.active_peers, 0);

        bridge
            .join_federation(make_peer("p", "P", GatewayTrustLevel::Full))
            .expect("test: join");
        bridge.leave_federation("p");

        let s = bridge.federation_stats();
        assert_eq!(s.peers_joined, 1);
        assert_eq!(s.peers_removed, 1);
        assert_eq!(s.active_peers, 0);
    }

    #[test]
    fn local_id() {
        let bridge = FederationBridge::new("my-federation".into(), 5);
        assert_eq!(bridge.local_id(), "my-federation");
    }
}
