// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! NGauge → TrustChain federation gating adapter (Phase F.1).
//!
//! TrustChain's [`trustchain::ca::trust_provider::TrustSignalProvider`]
//! trait is intentionally narrow — it returns a coarse `PeerTrustBand`
//! per peer fingerprint.  This adapter is the blockmatrix side of the
//! bridge: it owns a small lookup table (or pluggable closure) that
//! maps SHA-256 fingerprints to ngauge `PeerTrustSignals`, derives a
//! `TrustBand` via `PeerTrustSignals::trust_band`, and returns the
//! corresponding `PeerTrustBand`.
//!
//! Why a lookup table rather than a direct query of `NGaugeBridge`?
//! NGauge's `SwarmAnalytics` tracks shard-level popularity, not
//! per-peer trust.  Phase F.1 introduces the protocol seam — concrete
//! per-peer signal accumulation lives in a follow-up sprint.  For now
//! callers populate the table from whatever signal source they have
//! (test mocks, eventual `PeerMetrics` aggregator, etc.), and the
//! federation manager picks the band up through the trait.

#![cfg(feature = "intelligence")]

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use ngauge::{PeerTrustSignals, TrustBand};
use trustchain::ca::trust_provider::{
    PeerCertFingerprint, PeerTrustBand, TrustSignalProvider,
};

/// Adapter exposing ngauge-derived signals as a TrustChain
/// [`TrustSignalProvider`].
pub struct NGaugeTrustAdapter {
    /// Per-fingerprint signal cache.  Populated by the daemon (or by
    /// tests) as ngauge accumulates measurements.
    signals: Arc<RwLock<HashMap<PeerCertFingerprint, PeerTrustSignals>>>,
}

impl Default for NGaugeTrustAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NGaugeTrustAdapter {
    /// Construct an empty adapter.
    pub fn new() -> Self {
        Self {
            signals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert or overwrite the cached signals for a peer.
    pub async fn upsert(&self, peer: PeerCertFingerprint, signals: PeerTrustSignals) {
        self.signals.write().await.insert(peer, signals);
    }

    /// Remove the cached signals for a peer.
    pub async fn forget(&self, peer: &PeerCertFingerprint) {
        self.signals.write().await.remove(peer);
    }
}

#[async_trait]
impl TrustSignalProvider for NGaugeTrustAdapter {
    async fn trust_band_for(&self, peer: &PeerCertFingerprint) -> Option<PeerTrustBand> {
        let signals = self.signals.read().await.get(peer).cloned()?;
        match signals.trust_band() {
            TrustBand::Full => Some(PeerTrustBand::Full),
            TrustBand::Conditional => Some(PeerTrustBand::Conditional),
            TrustBand::Untrusted => {
                // The provider trait deliberately disallows returning
                // `Untrusted` — the byzantine override path inside the
                // federation manager owns that case.  Surface it as
                // `Conditional` so the federation manager downgrades
                // but doesn't reject outright.
                Some(PeerTrustBand::Conditional)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ngauge::capacity::CapacityMetrics;
    use ngauge::metrics::ActivityScore;
    use ngauge::organic_detection::TrafficClassification;

    fn high_activity() -> ActivityScore {
        ActivityScore {
            compute_score: 0.8,
            bandwidth_score: 0.8,
            latency_score: 0.7,
            receipt_density: 0.7,
        }
    }

    #[tokio::test]
    async fn returns_full_for_high_signals() {
        let adapter = NGaugeTrustAdapter::new();
        let fp = [0xAB; 32];
        let signals = PeerTrustSignals::new(
            high_activity(),
            CapacityMetrics::new(
                10_000_000,
                1_000_000,
                10 * 1024 * 1024 * 1024,
                100_000_000,
                0.99,
            ),
            TrafficClassification::Organic { confidence: 0.95 },
        );
        adapter.upsert(fp, signals).await;
        assert_eq!(adapter.trust_band_for(&fp).await, Some(PeerTrustBand::Full));
    }

    #[tokio::test]
    async fn returns_none_for_unknown_peer() {
        let adapter = NGaugeTrustAdapter::new();
        let fp = [0xCD; 32];
        assert_eq!(adapter.trust_band_for(&fp).await, None);
    }

    #[tokio::test]
    async fn returns_conditional_for_speculative_traffic() {
        let adapter = NGaugeTrustAdapter::new();
        let fp = [0xEE; 32];
        let signals = PeerTrustSignals::new(
            high_activity(),
            CapacityMetrics::new(
                10_000_000,
                1_000_000,
                10 * 1024 * 1024 * 1024,
                100_000_000,
                0.99,
            ),
            TrafficClassification::Speculative { confidence: 0.9 },
        );
        adapter.upsert(fp, signals).await;
        assert_eq!(
            adapter.trust_band_for(&fp).await,
            Some(PeerTrustBand::Conditional)
        );
    }
}
