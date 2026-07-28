// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! World-scoped fetch gate — the mounted boundary enforcer.
//!
//! `DefaultIsolationManager` is the dormant enforcer; this is the thin,
//! single-purpose surface the live node consults on the replication/fetch
//! path (VISION.md §5.5 — worlds are emergent, nestable networks; a node is
//! responsible for the locality of its own world).
//!
//! The gate binds one node's *home world* to the enforcer and answers one
//! question: **may this node fetch/replicate a shard belonging to
//! `shard_world`?** Same-world traffic passes (a strict no-op while only
//! [`GLOBAL_WORLD`](hypermesh_lib::GLOBAL_WORLD) exists); a shard belonging to
//! a foreign world is rejected before any transfer is attempted, logged at
//! `warn!`.
//!
//! This is intentionally permissive-for-same-world: until a second world
//! forms (P6), `check_fetch` can only ever be handed the node's own world, so
//! it always accepts. The gate *exists and is consulted*; it rejects genuine
//! cross-world traffic only, of which there is none in a single-world node.

use anyhow::Result;
use hypermesh_lib::{ContentHash, NetworkId};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{DefaultIsolationManager, IsolationManager, IsolationStats, IsolationViolation};
use crate::network::trust::NetworkType;

/// A mounted, world-scoped boundary gate over [`DefaultIsolationManager`].
pub struct WorldIsolationGate {
    /// The dormant enforcer, now mounted and configured for `local_world`.
    manager: Arc<DefaultIsolationManager>,
    /// This node's home world — the root world it always belongs to.
    local_world: NetworkId,
    /// The full set of worlds this node participates in — its home world plus
    /// any emergent child world it has been folded into as those form (P6,
    /// VISION.md §5.5). A fetch for a shard in ANY admitted world is legitimate
    /// same-world traffic; only a world absent from this set is foreign and
    /// rejected. Seeded with `local_world` at mount, so a single-world node
    /// admits exactly its home world (identical to P5 behaviour).
    admitted: Arc<RwLock<HashSet<NetworkId>>>,
}

impl WorldIsolationGate {
    /// Mount the enforcer for a node whose home world is `local_world`.
    ///
    /// Configures the isolation manager with the home world so that
    /// same-world probes pass the packet filter. `network_type` records the
    /// world's trust posture for observability only.
    pub async fn mount(local_world: NetworkId, network_type: NetworkType) -> Result<Self> {
        let manager = Arc::new(DefaultIsolationManager::new());
        manager
            .configure_network(local_world, network_type)
            .await?;
        let mut admitted = HashSet::new();
        admitted.insert(local_world);
        Ok(Self {
            manager,
            local_world,
            admitted: Arc::new(RwLock::new(admitted)),
        })
    }

    /// The node's home world.
    pub fn local_world(&self) -> NetworkId {
        self.local_world
    }

    /// Fold an emergent child world into this node's membership (P6).
    ///
    /// Called when a [`WorldManager`](../../../../ngauge) forms/joins a child
    /// world whose hot chunk this node holds: the node is now a legitimate
    /// mirror inside `world`, so a fetch for a shard belonging to `world` is
    /// same-world traffic and must be accepted. Idempotent. Registers the world
    /// with the underlying enforcer for audit symmetry.
    pub async fn admit_world(&self, world: NetworkId, network_type: NetworkType) -> Result<()> {
        {
            let mut admitted = self.admitted.write().await;
            if !admitted.insert(world) {
                return Ok(());
            }
        }
        // Best-effort enforcer registration; a duplicate config is not fatal.
        let _ = self.manager.configure_network(world, network_type).await;
        info!(
            "world-isolation: admitted world {} into node membership (home {})",
            world, self.local_world,
        );
        Ok(())
    }

    /// Drop an emergent child world from this node's membership — the inverse of
    /// [`admit_world`], used when a child world is merged back into its parent
    /// (P6 merge). The home world can never be dropped.
    pub async fn revoke_world(&self, world: NetworkId) {
        if world == self.local_world {
            return;
        }
        let mut admitted = self.admitted.write().await;
        if admitted.remove(&world) {
            info!(
                "world-isolation: revoked world {} from node membership (home {})",
                world, self.local_world,
            );
        }
    }

    /// Whether this node currently participates in `world`.
    pub async fn admits(&self, world: NetworkId) -> bool {
        self.admitted.read().await.contains(&world)
    }

    /// Consult the boundary enforcer before fetching/replicating a shard that
    /// belongs to `shard_world`.
    ///
    /// - **Home world** (`shard_world == local_world`) → runs the full enforcer
    ///   path (`validate_boundary`), exactly as P5 did — the single-world no-op.
    /// - **Admitted child world** → `Ok(())`. The node is a member of this
    ///   emergent world (it holds the migrated hot chunk), so the fetch is
    ///   legitimate same-world traffic, not a boundary crossing.
    /// - **Foreign world** (not admitted) → `Err(..)`, logged at `warn!`, with
    ///   the violation recorded in the enforcer's audit log.
    pub async fn check_fetch(
        &self,
        shard_world: NetworkId,
        shard_id: &ContentHash,
    ) -> Result<()> {
        // A world the node participates in (other than home) is same-world
        // traffic: accept without a boundary probe so a legitimate holder of a
        // migrated shard is never stranded.
        if shard_world != self.local_world && self.admits(shard_world).await {
            return Ok(());
        }
        match self
            .manager
            .validate_boundary(self.local_world, shard_world)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!(
                    "world-isolation: rejected cross-world fetch of shard {} \
                     (home world {}, shard world {})",
                    hex::encode(&shard_id.0[..4]),
                    self.local_world,
                    shard_world,
                );
                Err(e)
            }
        }
    }

    /// Recorded boundary violations (audit trail for tests / observability).
    pub async fn violations(&self) -> Vec<IsolationViolation> {
        self.manager.check_violations().await
    }

    /// Enforcer statistics (packets validated/rejected, violations).
    pub async fn stats(&self) -> IsolationStats {
        self.manager.get_stats().await
    }
}
