// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Replication / placement service — the intelligence-driven background loops
//! that keep hot shards replicated and steer where new replicas land.
//!
//! This is a **mechanical extraction** (P8) of the loops that previously lived
//! inline in the `connect` command's `start_network`. Behaviour is byte-for-byte
//! identical: same intervals (H3 demand feed 10s, H4/H5 propagation-weight +
//! replication-signal 15s, E.2 replication-poll 30s), same `DEFAULT_NETWORK`
//! defaults, same flat per-network replication, same dispersion source
//! selection.
//!
//! The service takes every handle it needs as an explicit field on
//! [`ReplicationService`] (the analytics/index/transport inputs)
//! rather than reaching back into the connect command's locals — the transport
//! bring-up owns those `Arc`s and hands the SAME instances here, so lock scopes
//! and sharing semantics are unchanged.
//!
//! The whole module is gated behind the `intelligence` feature (the loops
//! consume ngauge's routing/swarm analytics surface); a no-intelligence build
//! compiles it out entirely.

use std::sync::{Arc, Mutex};

use hypermesh_lib::NetworkId;

use crate::blockchain::propagation::BlockPropagator;
use crate::blockchain::sync_manager::SyncManager;
use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::shard_transport::StoqShardTransport;
use crate::network::swarm_provider::ShardLocationIndex;
use crate::network::{NetworkManager, SwarmDemandTracker};

mod feed;
mod poll;
mod propagation;

/// The handles the replication/placement loops operate over.
///
/// Every field is an `Arc` (or a `Copy` scalar) the transport bring-up already
/// holds; constructing this bundle and calling [`ReplicationService::spawn`]
/// hands those SAME instances to the loops. No new lock ordering is introduced —
/// each loop clones from these fields exactly as the inline code did.
pub struct ReplicationService {
    /// This node's matrix coordinate (used as the cold-start demand cell).
    pub coord: MatrixCoordinate,
    /// This node's id (skip-self guard when picking replication sources).
    pub node_id: String,
    /// Transport privacy mode carried for the loops' `NetworkType` selection.
    pub privacy_mode: PrivacyMode,
    /// Live per-shard demand snapshot source (H3 feed input).
    pub swarm_demand_tracker: Arc<SwarmDemandTracker>,
    /// Shared swarm analytics — the single source the loops read/write.
    pub ngauge_analytics: Arc<Mutex<ngauge::SwarmAnalytics>>,
    /// Block propagator whose weights H4 adjusts from routing intelligence.
    pub block_propagator: Arc<tokio::sync::Mutex<BlockPropagator>>,
    /// Live network manager — connected-peer/coordinate source.
    pub network: Arc<NetworkManager>,
    /// Sync manager — the source of the node's joined Network memberships. The
    /// loops key every `SwarmAnalytics` / `ShardLocationIndex` access on the
    /// node's joined network(s) rather than a hardcoded default, so writers and
    /// readers stay consistent. Under a single joined network every access uses
    /// that one canonical [`NetworkId`] (byte-identical to the pre-network path).
    pub sync_manager: Arc<tokio::sync::Mutex<SyncManager>>,
    /// Shared provider index (R12) — the E.2 loop reads providers and
    /// registers this node after a successful replica fetch.
    pub shard_location_index: Arc<ShardLocationIndex>,
    /// Shard transport used to pull extra replicas in the E.2 loop.
    pub shard_transport: Arc<StoqShardTransport>,
    /// Optional shared eBPF orchestrator (kernel-map feedback); `None` in the
    /// userspace-only tier.
    pub ebpf: Option<Arc<hypermesh_ebpf::HyperMeshEbpf>>,
}

impl ReplicationService {
    /// Spawn the three background loops.
    ///
    /// Order and await points match the original inline code: the H3 feed and
    /// H4/H5 propagation loops are fire-and-forget `tokio::spawn`s; the E.2
    /// replication-poll loop is `async` + fallible to match, so the single `?`
    /// here is the same one that previously propagated out of `start_network`.
    pub async fn spawn(self) -> anyhow::Result<()> {
        feed::spawn(&self);
        propagation::spawn(&self);
        poll::spawn(&self).await?;
        Ok(())
    }
}

/// Snapshot the node's joined Network memberships as canonical [`NetworkId`]s.
///
/// The loops call this once per tick (locking the sync manager briefly, then
/// dropping the guard before touching the analytics/index locks — no nested
/// lock ordering). An empty result means the node has joined no Network chain,
/// in which case the loops have nothing to replicate; under the normal
/// single-network bring-up this returns exactly the one network the node joined,
/// so every index access keys on that one canonical id.
pub(super) async fn joined_networks(
    sync_manager: &Arc<tokio::sync::Mutex<SyncManager>>,
) -> Vec<NetworkId> {
    sync_manager
        .lock()
        .await
        .active_networks()
        .iter()
        .map(|m| m.network_id)
        .collect()
}
