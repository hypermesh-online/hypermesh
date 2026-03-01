// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-scope data transfer proxy.
//!
//! Tracks active STOQ data transfers that span two different
//! [`BlockchainScope`]s (Device <-> Network). Each transfer has a lifecycle:
//! `Pending -> InProgress -> Completed | Failed`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use hypermesh_lib::BlockchainScope;

use crate::error::GatewayError;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Lifecycle state of a cross-scope transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// An active cross-scope data transfer managed by the proxy.
#[derive(Debug, Clone)]
pub struct ActiveTransfer {
    pub transfer_id: String,
    pub from_scope: BlockchainScope,
    pub to_scope: BlockchainScope,
    pub state: TransferState,
    pub bytes_transferred: u64,
    pub started_at: Instant,
}

/// Snapshot of bridge proxy statistics.
#[derive(Debug, Clone)]
pub struct BridgeProxyStatsSnapshot {
    pub transfers_initiated: u64,
    pub transfers_completed: u64,
    pub transfers_failed: u64,
    pub total_bytes_bridged: u64,
    pub active_transfers: usize,
}

// ---------------------------------------------------------------------------
// Internal stats
// ---------------------------------------------------------------------------

struct BridgeProxyStats {
    transfers_initiated: AtomicU64,
    transfers_completed: AtomicU64,
    transfers_failed: AtomicU64,
    total_bytes_bridged: AtomicU64,
}

impl BridgeProxyStats {
    fn new() -> Self {
        Self {
            transfers_initiated: AtomicU64::new(0),
            transfers_completed: AtomicU64::new(0),
            transfers_failed: AtomicU64::new(0),
            total_bytes_bridged: AtomicU64::new(0),
        }
    }
}

// ---------------------------------------------------------------------------
// ScopeBridgeProxy
// ---------------------------------------------------------------------------

/// Forwards STOQ data across blockchain scopes.
///
/// Callers [`start_transfer`](ScopeBridgeProxy::start_transfer) to create a
/// transfer, then [`record_bytes`](ScopeBridgeProxy::record_bytes) as data
/// flows, and finally [`complete_transfer`](ScopeBridgeProxy::complete_transfer)
/// or [`fail_transfer`](ScopeBridgeProxy::fail_transfer) to finalise it.
pub struct ScopeBridgeProxy {
    active_transfers: Arc<DashMap<String, ActiveTransfer>>,
    stats: Arc<BridgeProxyStats>,
}

impl ScopeBridgeProxy {
    /// Create a new proxy with no active transfers.
    pub fn new() -> Self {
        Self {
            active_transfers: Arc::new(DashMap::new()),
            stats: Arc::new(BridgeProxyStats::new()),
        }
    }

    /// Start a new cross-scope transfer.
    ///
    /// Returns the unique `transfer_id`.
    ///
    /// # Errors
    ///
    /// [`GatewayError::ScopeRouting`] if `from_scope == to_scope` (no bridge
    /// needed).
    pub fn start_transfer(
        &self,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
    ) -> Result<String, GatewayError> {
        if from_scope == to_scope {
            return Err(GatewayError::ScopeRouting(
                "same-scope transfer not needed".into(),
            ));
        }
        let transfer_id = Uuid::new_v4().to_string();
        let transfer = ActiveTransfer {
            transfer_id: transfer_id.clone(),
            from_scope,
            to_scope,
            state: TransferState::Pending,
            bytes_transferred: 0,
            started_at: Instant::now(),
        };
        self.active_transfers.insert(transfer_id.clone(), transfer);
        self.stats
            .transfers_initiated
            .fetch_add(1, Ordering::Relaxed);
        info!(
            "started cross-scope transfer {} ({:?} -> {:?})",
            transfer_id, from_scope, to_scope
        );
        Ok(transfer_id)
    }

    /// Record `bytes` transferred for an active transfer.
    ///
    /// Transitions the transfer from `Pending` to `InProgress` on the first
    /// call.
    ///
    /// # Errors
    ///
    /// [`GatewayError::ScopeRouting`] if the transfer does not exist or has
    /// already completed/failed.
    pub fn record_bytes(&self, transfer_id: &str, bytes: u64) -> Result<(), GatewayError> {
        let mut entry = self.active_transfers.get_mut(transfer_id).ok_or_else(|| {
            GatewayError::ScopeRouting(format!("transfer '{transfer_id}' not found"))
        })?;
        match entry.state {
            TransferState::Completed | TransferState::Failed => {
                return Err(GatewayError::ScopeRouting(format!(
                    "transfer '{transfer_id}' already finalised"
                )));
            }
            TransferState::Pending => {
                entry.state = TransferState::InProgress;
            }
            TransferState::InProgress => {}
        }
        entry.bytes_transferred += bytes;
        self.stats
            .total_bytes_bridged
            .fetch_add(bytes, Ordering::Relaxed);
        debug!(
            "transfer {} recorded {} bytes (total: {})",
            transfer_id, bytes, entry.bytes_transferred
        );
        Ok(())
    }

    /// Mark a transfer as successfully completed.
    ///
    /// # Errors
    ///
    /// [`GatewayError::ScopeRouting`] if the transfer does not exist.
    pub fn complete_transfer(&self, transfer_id: &str) -> Result<(), GatewayError> {
        let mut entry = self.active_transfers.get_mut(transfer_id).ok_or_else(|| {
            GatewayError::ScopeRouting(format!("transfer '{transfer_id}' not found"))
        })?;
        entry.state = TransferState::Completed;
        self.stats
            .transfers_completed
            .fetch_add(1, Ordering::Relaxed);
        info!(
            "transfer {} completed ({} bytes)",
            transfer_id, entry.bytes_transferred
        );
        Ok(())
    }

    /// Mark a transfer as failed.
    ///
    /// # Errors
    ///
    /// [`GatewayError::ScopeRouting`] if the transfer does not exist.
    pub fn fail_transfer(&self, transfer_id: &str, reason: &str) -> Result<(), GatewayError> {
        let mut entry = self.active_transfers.get_mut(transfer_id).ok_or_else(|| {
            GatewayError::ScopeRouting(format!("transfer '{transfer_id}' not found"))
        })?;
        entry.state = TransferState::Failed;
        self.stats.transfers_failed.fetch_add(1, Ordering::Relaxed);
        warn!(
            "transfer {} failed: {} ({} bytes transferred)",
            transfer_id, reason, entry.bytes_transferred
        );
        Ok(())
    }

    /// Get a snapshot of a transfer's current state.
    pub fn get_transfer(&self, transfer_id: &str) -> Option<ActiveTransfer> {
        self.active_transfers
            .get(transfer_id)
            .map(|e| e.value().clone())
    }

    /// Count of transfers currently in `Pending` or `InProgress` state.
    pub fn active_count(&self) -> usize {
        self.active_transfers
            .iter()
            .filter(|e| {
                matches!(
                    e.value().state,
                    TransferState::Pending | TransferState::InProgress
                )
            })
            .count()
    }

    /// Remove completed or failed transfers older than `max_age`.
    ///
    /// Returns the number of entries removed.
    pub fn cleanup(&self, max_age: Duration) -> usize {
        let now = Instant::now();
        let mut removed = 0usize;
        self.active_transfers.retain(|_, transfer| {
            let finalised = matches!(
                transfer.state,
                TransferState::Completed | TransferState::Failed
            );
            let expired = now.duration_since(transfer.started_at) > max_age;
            if finalised && expired {
                removed += 1;
                false
            } else {
                true
            }
        });
        if removed > 0 {
            debug!("cleaned up {} stale transfers", removed);
        }
        removed
    }

    /// Return a snapshot of proxy statistics.
    pub fn proxy_stats(&self) -> BridgeProxyStatsSnapshot {
        BridgeProxyStatsSnapshot {
            transfers_initiated: self.stats.transfers_initiated.load(Ordering::Relaxed),
            transfers_completed: self.stats.transfers_completed.load(Ordering::Relaxed),
            transfers_failed: self.stats.transfers_failed.load(Ordering::Relaxed),
            total_bytes_bridged: self.stats.total_bytes_bridged.load(Ordering::Relaxed),
            active_transfers: self.active_count(),
        }
    }
}

impl Default for ScopeBridgeProxy {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_transfer_returns_unique_id() {
        let proxy = ScopeBridgeProxy::new();
        let id1 = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start 1");
        let id2 = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start 2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn same_scope_transfer_rejected() {
        let proxy = ScopeBridgeProxy::new();
        let result = proxy.start_transfer(BlockchainScope::Device, BlockchainScope::Device);
        assert!(result.is_err());
    }

    #[test]
    fn record_bytes_transitions_to_in_progress() {
        let proxy = ScopeBridgeProxy::new();
        let id = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start");

        let t = proxy.get_transfer(&id).expect("test: get before");
        assert_eq!(t.state, TransferState::Pending);

        proxy.record_bytes(&id, 1024).expect("test: record");

        let t = proxy.get_transfer(&id).expect("test: get after");
        assert_eq!(t.state, TransferState::InProgress);
        assert_eq!(t.bytes_transferred, 1024);
    }

    #[test]
    fn record_bytes_accumulates() {
        let proxy = ScopeBridgeProxy::new();
        let id = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start");
        proxy.record_bytes(&id, 100).expect("test: first");
        proxy.record_bytes(&id, 200).expect("test: second");

        let t = proxy.get_transfer(&id).expect("test: get");
        assert_eq!(t.bytes_transferred, 300);
    }

    #[test]
    fn record_bytes_on_completed_fails() {
        let proxy = ScopeBridgeProxy::new();
        let id = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start");
        proxy.complete_transfer(&id).expect("test: complete");
        assert!(proxy.record_bytes(&id, 1).is_err());
    }

    #[test]
    fn complete_transfer() {
        let proxy = ScopeBridgeProxy::new();
        let id = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start");
        proxy.record_bytes(&id, 512).expect("test: bytes");
        proxy.complete_transfer(&id).expect("test: complete");

        let t = proxy.get_transfer(&id).expect("test: get");
        assert_eq!(t.state, TransferState::Completed);
        assert_eq!(proxy.proxy_stats().transfers_completed, 1);
    }

    #[test]
    fn fail_transfer() {
        let proxy = ScopeBridgeProxy::new();
        let id = proxy
            .start_transfer(BlockchainScope::Network, BlockchainScope::Device)
            .expect("test: start");
        proxy
            .fail_transfer(&id, "network timeout")
            .expect("test: fail");

        let t = proxy.get_transfer(&id).expect("test: get");
        assert_eq!(t.state, TransferState::Failed);
        assert_eq!(proxy.proxy_stats().transfers_failed, 1);
    }

    #[test]
    fn complete_nonexistent_fails() {
        let proxy = ScopeBridgeProxy::new();
        assert!(proxy.complete_transfer("no-such-id").is_err());
    }

    #[test]
    fn fail_nonexistent_fails() {
        let proxy = ScopeBridgeProxy::new();
        assert!(proxy.fail_transfer("no-such-id", "reason").is_err());
    }

    #[test]
    fn active_count_tracks_pending_and_in_progress() {
        let proxy = ScopeBridgeProxy::new();
        let id1 = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start 1");
        let id2 = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start 2");
        assert_eq!(proxy.active_count(), 2);

        proxy.complete_transfer(&id1).expect("test: complete");
        assert_eq!(proxy.active_count(), 1);

        proxy.fail_transfer(&id2, "reason").expect("test: fail");
        assert_eq!(proxy.active_count(), 0);
    }

    #[test]
    fn cleanup_removes_old_finalised_transfers() {
        let proxy = ScopeBridgeProxy::new();
        let id = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start");
        proxy.complete_transfer(&id).expect("test: complete");

        // Zero-duration max_age means everything is "old"
        let removed = proxy.cleanup(Duration::ZERO);
        assert_eq!(removed, 1);
        assert!(proxy.get_transfer(&id).is_none());
    }

    #[test]
    fn cleanup_preserves_active_transfers() {
        let proxy = ScopeBridgeProxy::new();
        let _id = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start");

        let removed = proxy.cleanup(Duration::ZERO);
        assert_eq!(removed, 0);
        assert_eq!(proxy.active_count(), 1);
    }

    #[test]
    fn proxy_stats_snapshot() {
        let proxy = ScopeBridgeProxy::new();
        let s = proxy.proxy_stats();
        assert_eq!(s.transfers_initiated, 0);
        assert_eq!(s.transfers_completed, 0);
        assert_eq!(s.transfers_failed, 0);
        assert_eq!(s.total_bytes_bridged, 0);
        assert_eq!(s.active_transfers, 0);

        let id = proxy
            .start_transfer(BlockchainScope::Device, BlockchainScope::Network)
            .expect("test: start");
        proxy.record_bytes(&id, 256).expect("test: bytes");
        proxy.complete_transfer(&id).expect("test: complete");

        let s = proxy.proxy_stats();
        assert_eq!(s.transfers_initiated, 1);
        assert_eq!(s.transfers_completed, 1);
        assert_eq!(s.total_bytes_bridged, 256);
    }

    #[test]
    fn default_impl() {
        let proxy = ScopeBridgeProxy::default();
        assert_eq!(proxy.active_count(), 0);
    }

    #[test]
    fn get_transfer_nonexistent_returns_none() {
        let proxy = ScopeBridgeProxy::new();
        assert!(proxy.get_transfer("missing").is_none());
    }
}
